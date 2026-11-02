//! Reading the single string literal a macro takes, and reporting errors on it.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote_spanned;

/// An error to report as a `compile_error!` at `span`.
pub struct Error {
    pub span: Span,
    pub message: String,
}

impl Error {
    pub fn to_compile_error(&self) -> TokenStream {
        let message = &self.message;
        quote_spanned!(self.span=> ::core::compile_error!(#message))
    }
}

/// Reads exactly one string literal (normal or raw) and returns its text and span.
pub fn string_literal(input: TokenStream, example: &str) -> Result<(String, Span), Error> {
    let mut tokens = input.into_iter();
    let first = tokens.next();
    let not_literal = |span| Error {
        span,
        message: format!("expected a string literal, like {example}"),
    };
    let literal = match first {
        Some(TokenTree::Literal(literal)) => literal,
        // `$lit` passed through a `macro_rules!` arrives wrapped in an invisible group.
        Some(TokenTree::Group(group)) if group.delimiter() == proc_macro2::Delimiter::None => {
            let span = group.span();
            let (text, inner_span) =
                string_literal(group.stream(), example).map_err(|e| Error { span, ..e })?;
            return match tokens.next() {
                None => Ok((text, inner_span)),
                Some(extra) => Err(Error {
                    span: extra.span(),
                    message: "expected only one string literal".into(),
                }),
            };
        }
        Some(other) => return Err(not_literal(other.span())),
        None => return Err(not_literal(Span::call_site())),
    };
    let span = literal.span();
    if let Some(extra) = tokens.next() {
        return Err(Error {
            span: extra.span(),
            message: "expected only one string literal".into(),
        });
    }
    let parsed = litrs::StringLit::try_from(literal).map_err(|_| not_literal(span))?;
    if !parsed.suffix().is_empty() {
        return Err(Error {
            span,
            message: "string literals can't have a suffix".into(),
        });
    }
    Ok((parsed.value().to_string(), span))
}

/// Runs `parse` on the macro's literal; an `Err` becomes a compile error on the literal.
pub fn expand(
    input: TokenStream,
    example: &str,
    parse: impl FnOnce(&str) -> Result<TokenStream, String>,
) -> TokenStream {
    expand_spanned(input, example, |text, _| parse(text))
}

/// Like [`expand`], but `parse` also gets the literal's span, for errors the
/// expansion itself raises later (such as `cfg`-gated `compile_error!`s).
pub fn expand_spanned(
    input: TokenStream,
    example: &str,
    parse: impl FnOnce(&str, Span) -> Result<TokenStream, String>,
) -> TokenStream {
    let (text, span) = match string_literal(input, example) {
        Ok(found) => found,
        Err(error) => return error.to_compile_error(),
    };
    match parse(&text, span) {
        Ok(tokens) => tokens,
        Err(message) => Error { span, message }.to_compile_error(),
    }
}

/// Whitespace rule shared by `dur!` and `size!`: single ASCII spaces between parts only.
pub fn check_spaces(s: &str) -> Result<(), String> {
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return Err("remove the spaces around the literal".into());
    }
    if s.contains("  ") || s.chars().any(|c| c.is_whitespace() && c != ' ') {
        return Err("use single spaces between parts".into());
    }
    Ok(())
}

/// A non-negative decimal number, `digits[.digits]`, at the start of a literal.
pub struct Decimal<'a> {
    /// The number as written.
    pub text: &'a str,
    pub int: &'a str,
    pub frac: &'a str,
}

fn digits(s: &str) -> usize {
    s.bytes().take_while(u8::is_ascii_digit).count()
}

/// The unit written after the number at the start of `s`, with its space if it has one.
fn unit_after(s: &str) -> &str {
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
    let space = usize::from(rest.starts_with(' '));
    let letters = rest[space..]
        .char_indices()
        .find(|(_, c)| !c.is_alphabetic())
        .map_or(rest.len() - space, |(i, _)| i);
    if letters == 0 {
        ""
    } else {
        &rest[..space + letters]
    }
}

/// Reads a decimal at the start of `s`, or `None` when `s` doesn't start with one.
/// Error messages suggest fixed literals that keep the unit written after the number.
pub fn decimal(s: &str) -> Result<Option<Decimal<'_>>, String> {
    if s.starts_with('.') {
        let number = &s[..s.len()
            - s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
                .len()];
        let unit = unit_after(s);
        return Err(format!(
            "write a digit before the point, like \"0{number}{unit}\""
        ));
    }
    let int_len = digits(s);
    if int_len == 0 {
        return Ok(None);
    }
    let mut len = int_len;
    let mut frac = "";
    if let Some(after_point) = s[int_len..].strip_prefix('.') {
        let frac_len = digits(after_point);
        if frac_len == 0 {
            let int = &s[..int_len];
            let unit = unit_after(s);
            return Err(format!(
                "write a digit after the point, like \"{int}{unit}\" or \"{int}.5{unit}\""
            ));
        }
        frac = &after_point[..frac_len];
        len += 1 + frac_len;
    }
    Ok(Some(Decimal {
        text: &s[..len],
        int: &s[..int_len],
        frac,
    }))
}

/// The error for a number followed by `c` where a unit should start.
pub fn unexpected_after(number: &str, c: char) -> String {
    match c {
        '_' | '\'' => {
            format!("digit separators like `{c}` aren't supported; write the number without them")
        }
        _ => format!("unexpected `{c}` after {number}"),
    }
}

/// Why a decimal times a unit isn't a whole number.
pub enum Scale {
    TooLarge,
    NotWhole,
}

impl Decimal<'_> {
    /// `self * unit` exactly, when that is a whole number that fits in `u128`.
    pub fn times(&self, unit: u128) -> Result<u128, Scale> {
        let whole = self
            .int
            .parse::<u128>()
            .ok()
            .and_then(|n| n.checked_mul(unit));
        let whole = whole.ok_or(Scale::TooLarge)?;
        let frac = self.frac.trim_end_matches('0');
        if frac.is_empty() {
            return Ok(whole);
        }
        // Every unit is below 1e20, so more significant fraction digits can't be whole.
        if frac.len() > 20 {
            return Err(Scale::NotWhole);
        }
        let denominator = 10u128.pow(frac.len() as u32);
        let numerator = frac.parse::<u128>().expect("at most 20 digits") * unit;
        if !numerator.is_multiple_of(denominator) {
            return Err(Scale::NotWhole);
        }
        whole
            .checked_add(numerator / denominator)
            .ok_or(Scale::TooLarge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn ts(s: &str) -> TokenStream {
        TokenStream::from_str(s).unwrap()
    }

    fn lit(s: &str) -> Result<String, String> {
        string_literal(ts(s), "\"1h\"")
            .map(|(text, _)| text)
            .map_err(|e| e.message)
    }

    #[test]
    fn accepts_plain_and_raw() {
        assert_eq!(lit("\"1h\""), Ok("1h".to_string()));
        assert_eq!(lit(r#"r"1h""#), Ok("1h".to_string()));
        assert_eq!(lit(r##"r#"1h"#"##), Ok("1h".to_string()));
    }

    #[test]
    fn rejects_non_literal_and_extra_tokens() {
        assert_eq!(
            lit("1"),
            Err("expected a string literal, like \"1h\"".into())
        );
        assert_eq!(
            lit("x"),
            Err("expected a string literal, like \"1h\"".into())
        );
        assert_eq!(
            lit(""),
            Err("expected a string literal, like \"1h\"".into())
        );
        assert_eq!(
            lit("\"a\" \"b\""),
            Err("expected only one string literal".into())
        );
    }

    #[test]
    fn rejects_suffix() {
        assert_eq!(
            lit("\"1h\"x"),
            Err("string literals can't have a suffix".into())
        );
    }

    #[test]
    fn space_rule() {
        assert_eq!(check_spaces("1h 30m"), Ok(()));
        assert_eq!(check_spaces("1h"), Ok(()));
        let around = Err("remove the spaces around the literal".to_string());
        let single = Err("use single spaces between parts".to_string());
        assert_eq!(check_spaces(" 1h"), around);
        assert_eq!(check_spaces("1h "), around);
        assert_eq!(check_spaces("1h\n"), around);
        assert_eq!(check_spaces("1h  30m"), single);
        assert_eq!(check_spaces("1h\t30m"), single);
    }

    #[test]
    fn expand_reports_on_literal() {
        let out = expand(ts("\"x\""), "\"1h\"", |_| Err("bad".into())).to_string();
        assert!(out.contains("compile_error"), "{out}");
        assert!(out.contains("\"bad\""), "{out}");
        let ok = expand(ts("\"x\""), "\"1h\"", |s| Ok(ts(&format!("{}", s.len())))).to_string();
        assert_eq!(ok, "1");
    }
}
