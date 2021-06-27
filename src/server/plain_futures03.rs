use std::io::{Error, ErrorKind, Result};
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::headers::RawHeaders;

use super::sans_io::{self, Read as InnerRead};

#[derive(Debug)]
pub enum Read {
    NewPart { headers: RawHeaders },
    Part(Bytes),
    PartEof,
}

pin_project! {
    pub struct FormData<S> {
        #[pin]
        stream: S,
        inner: sans_io::FormData,
    }
}

impl<S> FormData<S> {
    pub fn new(stream: S, boundary: &str) -> Self {
        let inner = sans_io::FormData::new(boundary);
        Self { stream, inner }
    }
}

impl<S> Stream for FormData<S>
where
    S: Stream<Item = Result<Bytes>>,
{
    type Item = Result<Read>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.inner.read() {
                Ok(InnerRead::NeedsWrite) => {
                    match Pin::new(&mut this.stream).poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Some(Ok(bytes))) => {
                            this.inner.write(bytes).expect("we've been told to write");

                            // continue
                        }
                        Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
                        Poll::Ready(None) => {
                            this.inner.write_eof();

                            // continue
                        }
                    };
                }
                Ok(InnerRead::NewPart { headers }) => {
                    return Poll::Ready(Some(Ok(Read::NewPart { headers })))
                }
                Ok(InnerRead::Part(bytes)) => return Poll::Ready(Some(Ok(Read::Part(bytes)))),
                Ok(InnerRead::PartEof) => return Poll::Ready(Some(Ok(Read::PartEof))),
                Ok(InnerRead::None) => {
                    // continue
                }
                Ok(InnerRead::Eof) => return Poll::Ready(None),
                Err(err) => return Poll::Ready(Some(Err(Error::new(ErrorKind::Other, err)))),
            }
        }
    }
}

impl<S> FusedStream for FormData<S>
where
    S: Stream<Item = Result<Bytes>>,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_eof()
    }
}
