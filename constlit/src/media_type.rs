use core::fmt;

/// A media type (MIME type), usually made with [`mime!`](crate::mime).
///
/// The type, subtype and parameter names are lowercase; parameter values are
/// stored unquoted, and a `charset` value is lowercase too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MediaType {
    essence: &'static str,
    slash: usize,
    params: &'static [(&'static str, &'static str)],
}

impl MediaType {
    #[doc(hidden)]
    pub const fn __new_unchecked(
        essence: &'static str,
        slash: usize,
        params: &'static [(&'static str, &'static str)],
    ) -> Self {
        Self {
            essence,
            slash,
            params,
        }
    }

    /// `type/subtype`, without parameters, like `"text/html"`.
    pub const fn essence(&self) -> &'static str {
        self.essence
    }

    /// The part before the `/`, like `"text"`.
    pub const fn type_(&self) -> &'static str {
        self.essence.split_at(self.slash).0
    }

    /// The part after the `/`, like `"html"`.
    pub const fn subtype(&self) -> &'static str {
        self.essence.split_at(self.slash + 1).1
    }

    /// The parameters in written order, as `(name, value)` pairs.
    pub const fn params(&self) -> &'static [(&'static str, &'static str)] {
        self.params
    }

    /// The value of parameter `name`, which matches ASCII case-insensitively.
    pub fn param(&self, name: &str) -> Option<&'static str> {
        self.params
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|&(_, value)| value)
    }

    /// Converts to a [`mime::Mime`].
    ///
    /// Fails only for parameter values the `mime` crate can't parse although
    /// RFC 9110 allows them: an empty quoted value (`a=""`) or one containing
    /// `"` or `\`.
    #[cfg(feature = "mime")]
    pub fn to_mime(&self) -> Result<mime::Mime, mime::FromStrError> {
        use alloc::string::ToString;
        self.to_string().parse()
    }
}

/// RFC 9110 `tchar`: the characters of a token.
fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.essence)?;
        for (name, value) in self.params {
            if !value.is_empty() && value.bytes().all(is_tchar) {
                write!(f, "; {name}={value}")?;
                continue;
            }
            write!(f, "; {name}=\"")?;
            for c in value.chars() {
                if c == '"' || c == '\\' {
                    f.write_str("\\")?;
                }
                write!(f, "{c}")?;
            }
            f.write_str("\"")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Buf([u8; 128], usize);

    impl core::fmt::Write for Buf {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0[self.1..self.1 + s.len()].copy_from_slice(s.as_bytes());
            self.1 += s.len();
            Ok(())
        }
    }

    fn shown(m: &MediaType) -> Buf {
        let mut buf = Buf([0; 128], 0);
        core::fmt::write(&mut buf, format_args!("{m}")).unwrap();
        buf
    }

    #[test]
    fn display() {
        let quoted =
            MediaType::__new_unchecked("multipart/form-data", 9, &[("boundary", "a b\"c")]);
        let buf = shown(&quoted);
        assert_eq!(
            &buf.0[..buf.1],
            br#"multipart/form-data; boundary="a b\"c""#
        );
        let token = MediaType::__new_unchecked("text/html", 4, &[("charset", "utf-8"), ("q", "")]);
        let buf = shown(&token);
        assert_eq!(&buf.0[..buf.1], br#"text/html; charset=utf-8; q="""#);
    }

    #[test]
    fn accessors() {
        const HTML: MediaType = MediaType::__new_unchecked("text/html", 4, &[("charset", "utf-8")]);
        const TYPE: &str = HTML.type_();
        assert_eq!(TYPE, "text");
        assert_eq!(HTML.subtype(), "html");
        assert_eq!(HTML.essence(), "text/html");
        assert_eq!(HTML.param("CHARSET"), Some("utf-8"));
        assert_eq!(HTML.param("x"), None);
        assert_eq!(HTML.params(), &[("charset", "utf-8")]);
    }
}
