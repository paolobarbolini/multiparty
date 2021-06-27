use bytes::{BufMut, Bytes, BytesMut};

/// Search for `needle` in `haystack`
pub fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    memchr::memmem::find(haystack, needle)
}

/// Search for a `needle` that sits in `haystack1` and may continue in `haystack2`
pub fn find_bytes_split(mut haystack1: &[u8], haystack2: &[u8], needle: &[u8]) -> Option<usize> {
    let mut i = 0;

    while !haystack1.is_empty() && haystack1.len() + haystack2.len() >= needle.len() {
        if starts_with_between(haystack1, haystack2, needle) {
            return Some(i);
        }

        haystack1 = &haystack1[1..];
        i += 1;
    }

    None
}

/// Determine if `(haystack1 + haystack2).starts_with(needle)`
pub fn starts_with_between(haystack1: &[u8], haystack2: &[u8], needle: &[u8]) -> bool {
    let skip1 = haystack1.len().min(needle.len());

    let (needle1, needle2) = needle.split_at(skip1);
    &haystack1[..skip1] == needle1 && haystack2.starts_with(needle2)
}

/// Join `bytes1` and `bytes2` into a single allocation
pub fn join_bytes(bytes1: Bytes, bytes2: Bytes) -> Bytes {
    if bytes1.is_empty() {
        bytes2
    } else if bytes2.is_empty() {
        bytes1
    } else {
        let mut buf = BytesMut::with_capacity(bytes1.len() + bytes2.len());
        buf.put(bytes1);
        buf.put(bytes2);
        buf.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_bytes() {
        assert_eq!(find_bytes(b"abcdefgh", b"abc"), Some(0));
        assert_eq!(find_bytes(b"abc", b"abc"), Some(0));
        assert_eq!(find_bytes(b"abcdefgh", b"bcde"), Some(1));
        assert_eq!(find_bytes(b"abcdefgh", b"bc"), Some(1));
    }

    #[test]
    fn search_bytes_split() {
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"abc"), Some(0));
        assert_eq!(find_bytes_split(b"abc", b"", b"abc"), Some(0));
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"bcde"), Some(1));
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"bc"), Some(1));
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"fh"), None);
    }

    #[test]
    fn join() {
        assert_eq!(
            join_bytes(Bytes::copy_from_slice(b"abcd"), Bytes::new()),
            Bytes::copy_from_slice(b"abcd")
        );
        assert_eq!(
            join_bytes(Bytes::new(), Bytes::copy_from_slice(b"efgh")),
            Bytes::copy_from_slice(b"efgh")
        );
        assert_eq!(
            join_bytes(
                Bytes::copy_from_slice(b"abcd"),
                Bytes::copy_from_slice(b"efgh")
            ),
            Bytes::copy_from_slice(b"abcdefgh")
        );
    }
}
