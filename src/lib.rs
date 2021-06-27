//! # multiparty
//!
//! Simple zero copy* streaming multipart decoder implementation.
//!
//! \* Except for streams yielding `Bytes` smaller than half the boundary length.
//!
//! ## Examples
//!
//! ```toml
//! multiparty = { version = "0.1", features = ["server", "futures03"] }
//! ```
//!
//! ```rust
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use multiparty::server::owned_futures03::FormData;
//! use futures_util::stream::TryStreamExt;
//!
//! # if false {
//! let boundary = todo!("A multipart/form-data boundary");
//! let stream = todo!("A Stream<Item = std::io::Result<Bytes>> + Unpin");
//! # }
//! # let boundary = "abcd";
//! # let content = "--abcd\r\ncontent-type: text/plain\r\ncontent-disposition: form-data; name=\"foo\"; filename=\"test.txt\"\r\n\r\nbar\r\n--abcd--";
//! # let stream = futures_util::stream::once(futures_util::future::ready(Ok(bytes::Bytes::from(content.as_bytes()))));
//! let mut multipart = FormData::new(stream, boundary);
//!
//! while let Some(mut part) = multipart.try_next().await? {
//!     let headers = part.raw_headers().parse()?;
//!     println!("name: {:?}", headers.name);
//! #   assert_eq!(headers.name, "foo");
//!     println!("filename: {:?}", headers.filename);
//! #   assert_eq!(headers.filename.as_deref(), Some("test.txt"));
//!     println!("content_type: {:?}", headers.content_type);
//! #   assert_eq!(headers.content_type.as_deref(), Some("text/plain"));
//!
//!     while let Some(bytes) = part.try_next().await? {
//!         println!("Read {} bytes from the current part", bytes.len());
//! #       assert_eq!(bytes, "bar".as_bytes());
//!     }
//!
//!     println!("Reached the end of this part");
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(not(feature = "server"))]
compile_error!("This version requires the `server` feature on");

mod boundary;
pub mod headers;
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub mod server;
mod utils;
