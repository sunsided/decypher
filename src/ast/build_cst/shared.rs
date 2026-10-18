use crate::error::{CypherError, ErrorKind, NumberReason, Span};

/// Decode escape sequences from a string literal's content (already stripped of quotes).
pub fn decode_string_content(content: &str, span: Span) -> (String, Option<CypherError>) {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('\'') => result.push('\''),
                Some('"') => result.push('"'),
                Some('b') | Some('B') => result.push('\u{0008}'),
                Some('f') | Some('F') => result.push('\u{000C}'),
                Some('n') | Some('N') => result.push('\n'),
                Some('r') | Some('R') => result.push('\r'),
                Some('t') | Some('T') => result.push('\t'),
                Some('u') | Some('U') => {
                    let mut hex = String::new();
                    let mut count = 0;
                    while count < 8 && chars.peek().is_some() {
                        let next = *chars.peek().unwrap();
                        if next.is_ascii_hexdigit() {
                            hex.push(chars.next().unwrap());
                            count += 1;
                        } else {
                            break;
                        }
                    }
                    if count == 4 || count == 8 {
                        if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(codepoint) {
                                result.push(c);
                            } else {
                                let err_sp = Span::new(span.start, span.end);
                                return (
                                    result,
                                    Some(CypherError {
                                        kind: ErrorKind::InvalidEscape {
                                            sequence: format!("\\u{}", hex),
                                        },
                                        span: err_sp,
                                        source_label: None,
                                        notes: Vec::new(),
                                        source: None,
                                    }),
                                );
                            }
                        }
                    } else {
                        let err_sp = Span::new(span.start, span.end);
                        return (
                            result,
                            Some(CypherError {
                                kind: ErrorKind::InvalidEscape {
                                    sequence: format!("\\u{}", hex),
                                },
                                span: err_sp,
                                source_label: None,
                                notes: Vec::new(),
                                source: None,
                            }),
                        );
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => {
                    result.push('\\');
                }
            }
        } else {
            result.push(ch);
        }
    }
    (result, None)
}

/// Parse the magnitude of an integer literal: decimal, `0x` hexadecimal,
/// `0o` octal, or legacy leading-zero octal (`017`).
///
/// The magnitude may exceed `i64::MAX` by one so that `-9223372036854775808`
/// can be represented once the sign is applied.
pub fn parse_integer_magnitude(s: &str) -> Result<u64, NumberReason> {
    use std::num::IntErrorKind;
    let s = s.trim();
    let (digits, radix) = if let Some(hex) = s.strip_prefix("0x").or(s.strip_prefix("0X")) {
        (hex, 16)
    } else if let Some(oct) = s.strip_prefix("0o").or(s.strip_prefix("0O")) {
        (oct, 8)
    } else if s.starts_with('0') && s.len() > 1 && s.chars().all(|c| c.is_ascii_digit()) {
        (&s[1..], 8)
    } else {
        (s, 10)
    };
    u64::from_str_radix(digits, radix).map_err(|e| match e.kind() {
        IntErrorKind::Empty => NumberReason::Empty,
        IntErrorKind::PosOverflow => NumberReason::Overflow,
        _ => NumberReason::InvalidDigit,
    })
}

/// Parse an integer literal that fits an `i64`.
pub fn parse_integer(s: &str) -> Option<i64> {
    parse_integer_magnitude(s)
        .ok()
        .and_then(|m| i64::try_from(m).ok())
}

/// Parse a floating-point number from a string.
pub fn parse_double(s: &str) -> Option<f64> {
    let val = s.trim().parse::<f64>().ok()?;
    if val.is_nan() || val.is_infinite() {
        None
    } else {
        Some(val)
    }
}
