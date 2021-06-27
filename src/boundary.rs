use bytes::Bytes;

/// A multipart boundary stored as `\r\n--{boundary}`
#[derive(Debug, Clone)]
pub struct Boundary(Bytes);

impl Boundary {
    pub fn new(boundary: &str) -> Self {
        Self(format!("\r\n--{}", boundary).into())
    }

    /// Equivalent to `format!("--{}", boundary)`
    pub fn with_dashes(&self) -> Bytes {
        self.0.slice("\r\n".len()..)
    }

    /// Equivalent to `format!("\r\n--{}", boundary)`
    pub fn with_new_line_and_dashes(&self) -> Bytes {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary() {
        let boundary = Boundary::new("abcd");
        assert_eq!(boundary.with_dashes(), "--abcd");
        assert_eq!(boundary.with_new_line_and_dashes(), "\r\n--abcd");
    }
}
