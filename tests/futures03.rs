#[cfg(all(feature = "server", feature = "futures03"))]
use std::future::Future;

#[cfg(all(feature = "server", feature = "futures03"))]
use bytes::{BufMut, Bytes, BytesMut};
use futures_core::FusedStream;
#[cfg(all(feature = "server", feature = "futures03"))]
use futures_util::stream::{self, StreamExt, TryStreamExt};
#[cfg(all(feature = "server", feature = "futures03"))]
use multiparty::server::owned_futures03::FormData;
#[cfg(all(feature = "server", feature = "futures03"))]
use multiparty::server::sans_io::Error;

#[cfg(all(feature = "server", feature = "futures03"))]
fn ready_yield_now_maybe<T>(t: T) -> impl Future<Output = T> {
    let future = async move {
        if fastrand::bool() {
            tokio::task::yield_now().await;
        }

        t
    };

    Box::pin(future)
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn empty() {
    let boundary = "--abcdef1234--";

    let s = stream::empty();
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn empty_bytes() {
    let boundary = "--abcdef1234--";

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::new())));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_bad_suffix() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}??\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        assert_eq!(
            parts.next().await.unwrap().unwrap_err().to_string(),
            Error::UnexpectedBoundarySuffix.to_string()
        );
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_bad_headers() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         stuff\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        assert_eq!(
            parts.next().await.unwrap().unwrap_err().to_string(),
            Error::Headers(httparse::Error::HeaderName).to_string()
        );
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_random_body() {
    let body_contents = (0..4096)
        .map(|_| fastrand::alphanumeric())
        .collect::<String>();

    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         {1}\r\n\
         --{0}--\r\n\
         ",
        boundary, body_contents
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, body_contents);

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_filename() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"; filename=\"something.txt\"\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert_eq!(headers1.filename.as_deref(), Some("something.txt"));
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_filename_plus() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; something; name=\"foo\"; filename=\"something.txt\"\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert_eq!(headers1.filename.as_deref(), Some("something.txt"));
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_multipart() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}\r\n\
         content-disposition: form-data; name=\"abcd\"\r\n\r\n\
         efgh\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        let mut part2 = parts.next().await.unwrap().unwrap();
        let headers2 = part2.raw_headers().parse().unwrap();
        assert_eq!(headers2.name, "abcd");
        assert!(headers2.filename.is_none());
        assert!(headers2.content_type.is_none());

        let bytes1 = part2.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "efgh".as_bytes());

        assert!(part2.next().await.is_none());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_multipart_skip1() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}\r\n\
         content-disposition: form-data; name=\"abcd\"\r\n\r\n\
         efgh\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let part1 = parts.next().await.unwrap().unwrap();
        drop(part1);
    }

    {
        let mut part2 = parts.next().await.unwrap().unwrap();
        let headers2 = part2.raw_headers().parse().unwrap();
        assert_eq!(headers2.name, "abcd");
        assert!(headers2.filename.is_none());
        assert!(headers2.content_type.is_none());

        let bytes1 = part2.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "efgh".as_bytes());

        assert!(part2.next().await.is_none());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_multipart_forgot1() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}\r\n\
         content-disposition: form-data; name=\"abcd\"\r\n\r\n\
         efgh\r\n\
         --{0}--\r\n\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    let mut part1 = parts.next().await.unwrap().unwrap();

    {
        let mut part2 = parts.next().await.unwrap().unwrap();
        let headers2 = part2.raw_headers().parse().unwrap();
        assert_eq!(headers2.name, "abcd");
        assert!(headers2.filename.is_none());
        assert!(headers2.content_type.is_none());

        let bytes1 = part2.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "efgh".as_bytes());

        assert!(part2.next().await.is_none());
    }

    assert!(part1.next().await.unwrap().is_err());

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_no_close() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn bytes_no_close_multipart() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}\r\n\
         content-disposition: form-data; name=\"abcd\"\r\n\r\n\
         efgh\
         ",
        boundary
    );

    let s = stream::once(ready_yield_now_maybe(Ok(Bytes::from(body))));
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let mut part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes1 = part1.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "bar".as_bytes());

        assert!(part1.next().await.is_none());
        assert!(part1.is_terminated());
    }

    {
        let mut part2 = parts.next().await.unwrap().unwrap();
        let headers2 = part2.raw_headers().parse().unwrap();
        assert_eq!(headers2.name, "abcd");
        assert!(headers2.filename.is_none());
        assert!(headers2.content_type.is_none());

        let bytes1 = part2.next().await.unwrap().unwrap();
        assert_eq!(bytes1, "efgh".as_bytes());

        assert!(part2.next().await.is_none());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn byte_at_a_time() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        boundary
    )
    .into_bytes();

    let s = stream::iter(body.into_iter().map(|b| Ok(Bytes::copy_from_slice(&[b]))))
        .then(ready_yield_now_maybe);
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes = part1
            .try_fold(BytesMut::new(), |mut acc, b| async move {
                acc.put(b);
                Ok(acc)
            })
            .await
            .unwrap();
        assert_eq!(bytes, "bar".as_bytes());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn byte_at_a_time_random_body() {
    let body_contents = (0..4096)
        .map(|_| fastrand::alphanumeric())
        .collect::<String>();

    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         {1}\r\n\
         --{0}--\r\n\
         ",
        boundary, body_contents
    )
    .into_bytes();

    let s = stream::iter(body.into_iter().map(|b| Ok(Bytes::copy_from_slice(&[b]))))
        .then(ready_yield_now_maybe);
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes = part1
            .try_fold(BytesMut::new(), |mut acc, b| async move {
                acc.put(b);
                Ok(acc)
            })
            .await
            .unwrap();
        assert_eq!(bytes, body_contents);
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn byte_at_a_time_spurious_empty() {
    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        boundary
    )
    .into_bytes();

    let s = stream::iter(
        body.into_iter()
            .map(|b| {
                vec![
                    Ok(Bytes::new()),
                    Ok(Bytes::copy_from_slice(&[b])),
                    Ok(Bytes::new()),
                ]
            })
            .flatten(),
    )
    .then(ready_yield_now_maybe);
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes = part1
            .try_fold(BytesMut::new(), |mut acc, b| async move {
                acc.put(b);
                Ok(acc)
            })
            .await
            .unwrap();
        assert_eq!(bytes, "bar".as_bytes());
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn many_bytes_at_a_time_random_body() {
    let body_contents = (0..4096)
        .map(|_| fastrand::alphanumeric())
        .collect::<String>();

    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         {1}\r\n\
         --{0}--\r\n\
         ",
        boundary, body_contents
    )
    .into_bytes();

    let s = stream::iter(body.chunks(32).map(|b| Ok(Bytes::copy_from_slice(b))))
        .then(ready_yield_now_maybe);
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes = part1
            .try_fold(BytesMut::new(), |mut acc, b| async move {
                acc.put(b);
                Ok(acc)
            })
            .await
            .unwrap();
        assert_eq!(bytes, body_contents);
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}

#[cfg(all(feature = "server", feature = "futures03"))]
#[tokio::test]
async fn many_bytes_at_a_time_random_body_no_close() {
    let body_contents = (0..4096)
        .map(|_| fastrand::alphanumeric())
        .collect::<String>();

    let boundary = "--abcdef1234--";
    let body = format!(
        "\
         --{0}\r\n\
         content-disposition: form-data; name=\"foo\"\r\n\r\n\
         {1}\
         ",
        boundary, body_contents
    )
    .into_bytes();

    let s = stream::iter(body.chunks(32).map(|b| Ok(Bytes::copy_from_slice(b))))
        .then(ready_yield_now_maybe);
    let mut parts = FormData::new(s, boundary);

    assert!(!parts.is_terminated());

    {
        let part1 = parts.next().await.unwrap().unwrap();
        let headers1 = part1.raw_headers().parse().unwrap();
        assert_eq!(headers1.name, "foo");
        assert!(headers1.filename.is_none());
        assert!(headers1.content_type.is_none());

        assert!(!part1.is_terminated());
        let bytes = part1
            .try_fold(BytesMut::new(), |mut acc, b| async move {
                acc.put(b);
                Ok(acc)
            })
            .await
            .unwrap();
        assert_eq!(bytes, body_contents);
    }

    {
        assert!(parts.next().await.is_none());
        assert!(parts.is_terminated());
    }
}
