//! Sans IO implementation of a multipart decoder.
//!
//! The user is responsible for feeding [`Bytes`] to [`FormData`] and
//! reading the decoded multipart data.
//!
//! This implementation is very low-level. Most use-cases will be
//! be better served by one of the high level wrappers from the
//! [`server`] module.
//!
//! [`server`]: crate::server

use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::mem;

use bytes::{Buf, Bytes};

use crate::boundary::Boundary;
use crate::headers::RawHeaders;
use crate::utils::{find_bytes, find_bytes_split, join_bytes, starts_with_between};

/// Sans IO multipart decoder
pub struct FormData {
    boundary: Boundary,
    bytes1: Bytes,
    bytes2: Bytes,

    state: State,
}

/// An item read from [`FormData`]
#[derive(Debug)]
pub enum Read {
    /// More data needs to be given to [`FormData`] before progress can be made.
    NeedsWrite,
    /// The beginning of a new part.
    NewPart {
        /// The headers inside the new part
        headers: RawHeaders,
    },
    /// [`Bytes`] from the current part.
    Part(Bytes),
    /// The current part has ended. The next call to read may yield a new part.
    PartEof,
    /// No data for this call. Call read again to make progress.
    None,
    /// The multipart stream has reached it's end. Subsequent calls to read will
    /// always return [`Read::Eof`].
    Eof,
}

/// An error while decoding the multipart stream
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The binary suffix is supposed to either be `\r\n` or `--`,
    /// but a different suffix was found.
    UnexpectedBoundarySuffix,
    /// The end of stream was reached on a part which isn't supposed to be truncated.
    UnexpectedEof,
    /// An error was returned by the headers decoder.
    Headers(httparse::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedBoundarySuffix => f.write_str("unexpected boundary suffix"),
            Self::UnexpectedEof => f.write_str("unexpected eof"),
            Self::Headers(_) => f.write_str("header parsing error"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::UnexpectedBoundarySuffix | Self::UnexpectedEof => None,
            Self::Headers(err) => Some(err),
        }
    }
}

/// Internal state of [`FormData`]
#[derive(PartialEq)]
enum State {
    Uninit,
    BoundarySuffix,
    Headers,
    Part,
    WriteEof,
    Eof,
}

impl FormData {
    /// Create a new instance of [`FormData`] with a boundary of `boundary`.
    pub fn new(boundary: &str) -> Self {
        let boundary = Boundary::new(boundary);
        Self {
            boundary,
            bytes1: Bytes::new(),
            bytes2: Bytes::new(),
            state: State::Uninit,
        }
    }

    /// Add more [`Bytes`] to the internal state.
    ///
    /// In order to achieve 0 copy decoding `bytes` should have a
    /// length `>= boundary.len() + 4`. Smaller `bytes` are still
    /// accepted, but might require the decoder to do more work.
    ///
    /// Returns `Err(bytes)` if this `FormData` isn't expecting
    /// more bytes.
    pub fn write(&mut self, bytes: Bytes) -> Result<(), Bytes> {
        if matches!(self.state, State::WriteEof | State::Eof) {
            // It doesn't make sense to write after reaching eof
            Err(bytes)
        } else if self.bytes1.is_empty() {
            self.bytes1 = bytes;
            Ok(())
        } else if self.bytes2.is_empty() {
            self.bytes2 = bytes;
            Ok(())
        } else {
            // No space to put `bytes`
            Err(bytes)
        }
    }

    /// Signal to [`FormData`] that no more calls to [`FormData::write`] are
    /// going to be made, as EOF for the multipart bytes stream has been reached.
    pub fn write_eof(&mut self) {
        self.state = if self.state == State::Part {
            State::WriteEof
        } else {
            State::Eof
        }
    }

    #[cfg(feature = "futures03")]
    pub(super) fn is_eof(&self) -> bool {
        self.state == State::Eof
    }

    /// Get a new item of multipart data.
    pub fn read(&mut self) -> Result<Read, Error> {
        macro_rules! needs_write {
            () => {
                match self.state {
                    State::WriteEof | State::Eof => {
                        self.state = State::Eof;
                        Ok(Read::Eof)
                    }
                    _ => Ok(Read::NeedsWrite),
                }
            };
        }

        macro_rules! needs_write_while_parsing {
            () => {
                match self.state {
                    State::WriteEof | State::Eof => {
                        self.state = State::Eof;
                        Err(Error::UnexpectedEof)
                    }
                    _ => Ok(Read::NeedsWrite),
                }
            };
        }

        if self.bytes1.is_empty() {
            debug_assert!(self.bytes2.is_empty());

            return needs_write!();
        }

        match self.state {
            State::Uninit => {
                let boundary = self.boundary.with_dashes();

                match self.read_until_boundary(&boundary) {
                    Some((bytes, true)) => {
                        drop(bytes);

                        self.skip(boundary.len());
                        self.state = State::BoundarySuffix;
                        Ok(Read::None)
                    }
                    Some((_, false)) | None => {
                        needs_write!()
                    }
                }
            }
            State::BoundarySuffix => {
                if starts_with_between(&self.bytes1, &self.bytes2, b"\r\n") {
                    // There's another part after this one
                    self.skip(2);
                    self.state = State::Headers;

                    Ok(Read::None)
                } else if starts_with_between(&self.bytes1, &self.bytes2, b"--") {
                    // There are no more parts
                    self.state = State::Eof;
                    Ok(Read::Eof)
                } else if self.bytes1.len() + self.bytes2.len() < 2 {
                    needs_write_while_parsing!()
                } else {
                    Err(Error::UnexpectedBoundarySuffix)
                }
            }
            State::Headers => {
                let mut headers = [httparse::EMPTY_HEADER; 8];

                match httparse::parse_headers(&self.bytes1, &mut headers) {
                    Ok(httparse::Status::Complete((read, headers))) => {
                        let headers = headers
                            .iter()
                            .map(|header| {
                                let name = self.bytes1.slice_ref(header.name.as_bytes());
                                let value = self.bytes1.slice_ref(header.value);
                                (name, value)
                            })
                            .collect::<Vec<_>>();

                        self.skip(read);
                        self.state = State::Part;

                        let headers = RawHeaders::new(headers);
                        Ok(Read::NewPart { headers })
                    }
                    Ok(httparse::Status::Partial) => {
                        self.set_need_bytes2();
                        needs_write_while_parsing!()
                    }
                    Err(err) => Err(Error::Headers(err)),
                }
            }
            State::Part => {
                let boundary = self.boundary.with_new_line_and_dashes();

                match self.read_until_boundary(&boundary) {
                    Some((bytes, true)) => {
                        if bytes.is_empty() {
                            self.skip(boundary.len());
                            self.state = State::BoundarySuffix;
                            Ok(Read::PartEof)
                        } else {
                            Ok(Read::Part(bytes))
                        }
                    }
                    Some((bytes, false)) => Ok(Read::Part(bytes)),
                    None => {
                        needs_write!()
                    }
                }
            }
            State::WriteEof => {
                let boundary = self.boundary.with_new_line_and_dashes();

                match self.read_until_boundary(&boundary) {
                    Some((bytes, _)) if !bytes.is_empty() => Ok(Read::Part(bytes)),
                    _ => {
                        let bytes =
                            join_bytes(mem::take(&mut self.bytes1), mem::take(&mut self.bytes2));

                        self.state = State::Eof;
                        Ok(Read::Part(bytes))
                    }
                }
            }
            State::Eof => Ok(Read::Eof),
        }
    }

    /// Read bytes from the internal state.
    /// Returns:
    ///
    /// * `Some((Bytes, true))` if the `boundary` has been found.
    ///   `Bytes` contain bytes until the start of the `boundary`.
    /// * `Some((Bytes, false))` if the `boundary` hasn't been found.
    /// * `None` if more bytes are needed.
    fn read_until_boundary(&mut self, boundary: &[u8]) -> Option<(Bytes, bool)> {
        debug_assert!(!self.bytes1.is_empty());
        debug_assert!(!boundary.is_empty());

        if self.bytes1.len() >= boundary.len() {
            // `bytes1 >= boundary`, so we can use the normal algorithm for searching for the boundary

            match find_bytes(&self.bytes1, boundary) {
                Some(i) => {
                    // Boundary starts at `i`
                    Some((self.bytes1.split_to(i), true))
                }
                None => {
                    // No full boundary could be found. Return `self.bytes1` except for the last `boundary.len()) - 1` bytes
                    let bytes = self.bytes1.split_to(self.bytes1.len() - boundary.len() + 1);
                    Some((bytes, false))
                }
            }
        } else {
            // `bytes1 < boundary`, we have to get smart

            let bytes12_len = self.bytes1.len() + self.bytes2.len();
            if bytes12_len >= boundary.len() {
                // `bytes1 + bytes2 >= boundary`

                match find_bytes_split(&self.bytes1, &self.bytes2, boundary) {
                    Some(i) => {
                        // Boundary starts at `i` inside `bytes1`
                        Some((self.bytes1.split_to(i), true))
                    }
                    None => {
                        // No boundary between `bytes1` and `bytes2`

                        // Skip at most `(self.bytes1.len() + self.bytes2.len()) - boundary.len() + 1`
                        let to_skip = bytes12_len - boundary.len() + 1;
                        let bytes = if to_skip < self.bytes1.len() {
                            self.bytes1.split_to(to_skip)
                        } else {
                            mem::replace(&mut self.bytes1, mem::take(&mut self.bytes2))
                        };
                        Some((bytes, false))
                    }
                }
            } else {
                // We need `bytes2`
                self.set_need_bytes2();
                None
            }
        }
    }

    /// Skip `len` bytes from the internal [`Bytes`].
    fn skip(&mut self, len: usize) {
        debug_assert!((self.bytes1.len() + self.bytes2.len()) >= len);

        if self.bytes1.len() > len {
            self.bytes1.advance(len);
        } else {
            let bytes1 = mem::replace(&mut self.bytes1, mem::take(&mut self.bytes2));
            self.bytes1.advance(len - bytes1.len());
        }
    }

    /// Prepare space in [`FormData`] for more [`Bytes`] to be written.
    fn set_need_bytes2(&mut self) {
        self.bytes1 = join_bytes(mem::take(&mut self.bytes1), mem::take(&mut self.bytes2));
    }
}
