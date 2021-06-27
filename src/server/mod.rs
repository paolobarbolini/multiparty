//! Multipart decoder implementations

#[cfg(feature = "futures03")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures03")))]
pub mod owned_futures03;
#[cfg(feature = "futures03")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures03")))]
pub(super) mod plain_futures03;
pub mod sans_io;
