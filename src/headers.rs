//! Headers

use std::error::Error as StdError;
use std::fmt::{self, Debug, Display};
use std::str;

use bytes::Bytes;

/// Raw unparsed headers
#[derive(Debug, Clone)]
pub struct RawHeaders {
    headers: Vec<(Bytes, Bytes)>,
}

impl RawHeaders {
    pub(crate) fn new(headers: Vec<(Bytes, Bytes)>) -> Self {
        Self { headers }
    }

    /// Parse the `Content-Disposition` and the `Content-Type` headers.
    pub fn parse(&self) -> Result<Headers, Error> {
        let (name, filename) = self.parse_content_disposition()?;
        let name = name.to_string();
        let filename = filename.map(|filename| filename.to_string());

        let content_type = self.parse_content_type()?;
        let content_type = content_type.map(|content_type| content_type.to_string());

        Ok(Headers {
            name,
            filename,
            content_type,
        })
    }

    fn parse_content_disposition(&self) -> Result<(&str, Option<&str>), Error> {
        let content_disposition = self
            .header("content-disposition")
            .ok_or(Error(InnerError::ContentDispositionNotFound))?;

        let content_disposition = str::from_utf8(content_disposition)
            .map_err(|_| Error(InnerError::ContentDispositionUtf8))?;

        let content_disposition = content_disposition
            .strip_prefix("form-data")
            .ok_or(Error(InnerError::ContentDispositionNotFormData))?;

        // Parse the `name` and `filename` from the content-disposition
        let mut name = None;
        let mut filename = None;

        for param in content_disposition.split(';').skip(1) {
            let param = param.trim();

            let mut splitter = param.split('=');
            let param_name = splitter.next().expect("always Some");

            if param_name != "name" && param_name != "filename" {
                continue;
            }

            let param_value = splitter
                .next()
                .ok_or(Error(InnerError::InvalidContentDispositionParam))?;
            let param_value = param_value.trim_matches(|c: char| c.is_whitespace() || c == '"');

            if param_name == "name" {
                name = Some(param_value);
            } else {
                filename = Some(param_value);
            }
        }

        let name = name.ok_or(Error(InnerError::NoContentDispositionName))?;

        Ok((name, filename))
    }

    fn parse_content_type(&self) -> Result<Option<&str>, Error> {
        match self.header("content-type") {
            Some(value) => {
                let value =
                    str::from_utf8(value).map_err(|_| Error(InnerError::ContentTypeUtf8))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn header(&self, name: &str) -> Option<&Bytes> {
        let name = name.as_bytes();
        self.headers
            .iter()
            .find(|(name_, _value)| name_.eq_ignore_ascii_case(name))
            .map(|(_name, value)| value)
    }
}

/// Parsed `Content-Disposition` and `Content-Type` headers.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Headers {
    /// The `name` parameter of the `Content-Disposition` header.
    pub name: String,
    /// The optional `filename` parameter of the `Content-Disposition` header.
    pub filename: Option<String>,
    /// The value of the optional `Content-Type` header.
    pub content_type: Option<String>,
}

/// Error encountered while parsing the `Content-Disposition` and `Content-Type` headers.
#[derive(Debug, PartialEq)]
pub struct Error(InnerError);

#[derive(Debug, PartialEq)]
enum InnerError {
    ContentDispositionNotFound,
    ContentDispositionUtf8,
    ContentDispositionNotFormData,
    InvalidContentDispositionParam,
    NoContentDispositionName,
    ContentTypeUtf8,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            InnerError::ContentDispositionNotFound => {
                f.write_str("Content-Disposition header not found")
            }
            InnerError::ContentDispositionUtf8 => {
                f.write_str("Content-Disposition header isn't valid utf-8")
            }
            InnerError::ContentDispositionNotFormData => {
                f.write_str("Content-Disposition doesn't begin with 'form-data'")
            }
            InnerError::InvalidContentDispositionParam => {
                f.write_str("Invalid Content-Disposition parameter")
            }
            InnerError::NoContentDispositionName => {
                f.write_str("Content-Disposition is missing the name parameter")
            }
            InnerError::ContentTypeUtf8 => f.write_str("Content-Type header isn't valid utf-8"),
        }
    }
}

impl StdError for Error {}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn ascii() {
        let headers = vec![
            (
                Bytes::from_static(b"Content-Disposition"),
                Bytes::from_static(b"form-data; name=\"abcd\"; filename=\"test.txt\""),
            ),
            (
                Bytes::from_static(b"Content-Type"),
                Bytes::from_static(b"text/plain"),
            ),
        ];
        let headers = RawHeaders::new(headers);

        let parsed = headers.parse().unwrap();
        assert_eq!(parsed.name, "abcd");
        assert_eq!(parsed.filename.as_deref(), Some("test.txt"));
        assert_eq!(parsed.content_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn ascii_no_cd() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(b"form-data; name=\"abcd\"; filename=\"test.txt\""),
        )];
        let headers = RawHeaders::new(headers);

        let parsed = headers.parse().unwrap();
        assert_eq!(parsed.name, "abcd");
        assert_eq!(parsed.filename.as_deref(), Some("test.txt"));
        assert!(parsed.content_type.is_none());
    }

    #[test]
    fn ascii_no_ct_no_filename() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(b"form-data; name=\"abcd\""),
        )];
        let headers = RawHeaders::new(headers);

        let parsed = headers.parse().unwrap();
        assert_eq!(parsed.name, "abcd");
        assert!(parsed.filename.is_none());
        assert!(parsed.content_type.is_none());
    }

    #[test]
    fn ascii_bad_cd() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(b"duck; name=\"abcd\""),
        )];
        let headers = RawHeaders::new(headers);

        assert_eq!(
            headers.parse(),
            Err(Error(InnerError::ContentDispositionNotFormData))
        );
    }

    #[test]
    fn ascii_bad_cd2() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(b"form-data"),
        )];
        let headers = RawHeaders::new(headers);

        assert_eq!(
            headers.parse(),
            Err(Error(InnerError::NoContentDispositionName))
        );
    }

    #[test]
    fn ascii_bad_cd_param() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(b"form-data; name"),
        )];
        let headers = RawHeaders::new(headers);

        assert_eq!(
            headers.parse(),
            Err(Error(InnerError::InvalidContentDispositionParam))
        );
    }

    #[test]
    fn ascii_cd_no_name() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(b"form-data; filename=\"test.txt\""),
        )];
        let headers = RawHeaders::new(headers);

        assert_eq!(
            headers.parse(),
            Err(Error(InnerError::NoContentDispositionName))
        );
    }

    #[test]
    fn no_cd() {
        let headers = vec![(
            Bytes::from_static(b"Content-Type"),
            Bytes::from_static(b"text/plain"),
        )];
        let headers = RawHeaders::new(headers);

        assert_eq!(
            headers.parse(),
            Err(Error(InnerError::ContentDispositionNotFound))
        );
    }

    #[test]
    fn cd_not_utf8() {
        let headers = vec![(
            Bytes::from_static(b"Content-Disposition"),
            Bytes::from_static(&[255, 255, 255]),
        )];
        let headers = RawHeaders::new(headers);

        assert_eq!(
            headers.parse(),
            Err(Error(InnerError::ContentDispositionUtf8))
        );
    }

    #[test]
    fn ct_not_utf8() {
        let headers = vec![
            (
                Bytes::from_static(b"Content-Disposition"),
                Bytes::from_static(b"form-data; name=\"abcd\"; filename=\"test.txt\""),
            ),
            (
                Bytes::from_static(b"Content-Type"),
                Bytes::from_static(&[255, 255, 255]),
            ),
        ];
        let headers = RawHeaders::new(headers);

        assert_eq!(headers.parse(), Err(Error(InnerError::ContentTypeUtf8)));
    }
}
