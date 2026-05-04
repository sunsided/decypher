use crate::error::{CypherError, ErrorKind, Span};

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

/// Parse an integer from a string, handling 0x hex and 0-prefixed octal.
pub fn parse_integer(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).ok()
    } else if s.starts_with('0') && s.len() > 1 && s.chars().all(|c| c.is_ascii_digit()) {
        i64::from_str_radix(s, 8).ok()
    } else {
        s.parse::<i64>().ok()
    }
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
