# multiparty

[![crates.io](https://img.shields.io/crates/v/multiparty.svg)](https://crates.io/crates/multiparty)
[![Documentation](https://docs.rs/multiparty/badge.svg)](https://docs.rs/multiparty)
[![dependency status](https://deps.rs/crate/multiparty/0.1.0/status.svg)](https://deps.rs/crate/multiparty/0.1.0)
[![Rustc Version 1.45.2+](https://img.shields.io/badge/rustc-1.45.2+-lightgray.svg)](https://blog.rust-lang.org/2020/07/16/Rust-1.45.0.html)
[![CI](https://github.com/paolobarbolini/multiparty/workflows/CI/badge.svg)](https://github.com/paolobarbolini/multiparty/actions?query=workflow%3ACI)
[![codecov](https://codecov.io/gh/paolobarbolini/multiparty/branch/main/graph/badge.svg?token=K0YPC21N8D)](https://codecov.io/gh/paolobarbolini/multiparty)

Simple zero copy* streaming multipart decoder implementation.

Also exposes the underlying Sans IO decoder, for use outside of
the `futures` 0.3 ecosystem.

## Examples

```toml
multiparty = { version = "0.1", features = ["server", "futures03"] }
```

```rust
use multiparty::server::owned_futures03::FormData;
use futures_util::stream::TryStreamExt;

let boundary = todo!("A multipart/form-data boundary");
let stream = todo!("A Stream<Item = std::io::Result<Bytes>> + Unpin");
let mut multipart = FormData::new(stream, boundary);

while let Some(mut part) = multipart.try_next().await? {
    let headers = part.raw_headers().parse()?;
    println!("name: {:?}", headers.name);
    println!("filename: {:?}", headers.filename);
    println!("content_type: {:?}", headers.content_type);

    while let Some(bytes) = part.try_next().await? {
        println!("Read {} bytes from the current part", bytes.len());
    }

    println!("Reached the end of this part");
}
```

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.

---

\* Except for streams yielding `Bytes` smaller than half the boundary length.
