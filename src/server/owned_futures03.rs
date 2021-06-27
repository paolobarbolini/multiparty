//! `futures` `Stream` 0.3 high-level multipart decoder.
//!
//! NOTE: Currently requires the stream to also be [`Unpin`].

use std::fmt::{self, Debug};
use std::io::{Error, ErrorKind, Result};
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::stream::{FusedStream, Stream};
use try_lock::TryLock;

use super::plain_futures03::{self, Read};
use crate::headers::RawHeaders;

/// A `Stream` of multipart/form-data parts.
///
/// Yields [`Part`].
pub struct FormData<S> {
    inner: Arc<TryLock<Option<plain_futures03::FormData<S>>>>,
}

/// A single "part" of a `multipart/form-data` body.
///
/// Yielded by the [`FormData`] `Stream`.
pub struct Part<S> {
    headers: RawHeaders,

    inner: Option<Arc<TryLock<Option<plain_futures03::FormData<S>>>>>,
}

impl<S> FormData<S> {
    /// Construct a new `FormData` from a `Stream<Item = std::io::Result<Bytes>> + Unpin` and a `boundary`.
    pub fn new(stream: S, boundary: &str) -> Self {
        let inner_form = plain_futures03::FormData::new(stream, boundary);
        Self {
            inner: Arc::new(TryLock::new(Some(inner_form))),
        }
    }
}

impl<S> Stream for FormData<S>
where
    S: Stream<Item = Result<Bytes>> + Unpin,
{
    type Item = Result<Part<S>>;

    /// Poll the next [`Part`] in this multipart stream.
    ///
    /// Calling this method invalidates any previous [`Part`] polled from this
    /// instance of `FormData`, meaning that any subsequent attempts at
    /// polling `Bytes` from those [`Part`]s will wield an error.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match Arc::get_mut(&mut self.inner) {
            Some(_) => {
                // We have exclusive access to inner
            }
            None => {
                // An old `Part` has been kept around
                let inner = match self.inner.try_lock() {
                    Some(mut inner) => mem::take(&mut *inner),
                    None => {
                        // Something is holding the lock, but it should release it soon
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                };

                // We took body out of the other `Part`'s `Arc`, leaving a `None` in its place,
                // now make a new `Arc`
                self.inner = Arc::new(TryLock::new(inner));
            }
        };
        let mut inner = self.inner.try_lock().expect("TryLock was mem::forgotten");
        let inner = inner.as_mut().expect("inner should never be None");

        match Pin::new(inner).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(Read::NewPart { headers }))) => {
                let inner = Arc::clone(&self.inner);
                Poll::Ready(Some(Ok(Part {
                    headers,
                    inner: Some(inner),
                })))
            }
            Poll::Ready(Some(Ok(Read::Part(_)))) | Poll::Ready(Some(Ok(Read::PartEof))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}

impl<S> FusedStream for FormData<S>
where
    S: Stream<Item = Result<Bytes>> + Unpin,
{
    fn is_terminated(&self) -> bool {
        match self.inner.try_lock() {
            Some(inner) => match &*inner {
                Some(inner) => inner.is_terminated(),
                None => false,
            },
            None => false,
        }
    }
}

impl<S> Debug for FormData<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FormData").finish()
    }
}

impl<S> Part<S> {
    /// Access the raw headers of this [`Part`].
    pub fn raw_headers(&self) -> &RawHeaders {
        &self.headers
    }
}

impl<S> Stream for Part<S>
where
    S: Stream<Item = Result<Bytes>> + Unpin,
{
    type Item = Result<Bytes>;

    /// Poll [`Bytes`] from this `Part`'s body.
    ///
    /// This method yields an error if this is the non last `Part` yielded
    /// by the [`FormData`] that yielded this part.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner_arc = match &self.inner {
            Some(inner_arc) => inner_arc,
            None => {
                // If `self.inner` is `None`, this `Part` has been exhausted
                return Poll::Ready(None);
            }
        };

        let mut inner_ = match inner_arc.try_lock() {
            Some(inner) => inner,
            None => {
                // If something else is playing with the lock this `Part` isn't the last one
                return Poll::Ready(Some(Err(Error::new(
                    ErrorKind::Other,
                    "Tried to poll data from the not last Part",
                ))));
            }
        };

        let inner = match &mut *inner_ {
            Some(inner) => inner,
            None => {
                // `inner` was stolen from this `Part`, so it isn't the last one
                drop(inner_);
                self.inner = None;

                return Poll::Ready(Some(Err(Error::new(
                    ErrorKind::Other,
                    "Tried to poll data from the not last Part",
                ))));
            }
        };

        match Pin::new(inner).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(Read::Part(bytes)))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(Some(Ok(Read::PartEof))) | Poll::Ready(None) => {
                drop(inner_);

                self.inner = None;
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(Some(Ok(Read::NewPart { .. }))) => unreachable!(),
        }
    }
}

impl<S> FusedStream for Part<S>
where
    S: Stream<Item = Result<Bytes>> + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assertions() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_unpin<T: Unpin>() {}

        struct PerfectStream;

        impl Stream for PerfectStream {
            type Item = Result<Bytes>;

            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        assert_send::<FormData<PerfectStream>>();
        assert_sync::<FormData<PerfectStream>>();
        assert_unpin::<FormData<PerfectStream>>();

        assert_send::<Part<PerfectStream>>();
        assert_sync::<Part<PerfectStream>>();
        assert_unpin::<Part<PerfectStream>>();
    }
}
