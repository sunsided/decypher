//! Hand-written lexer for openCypher source text.
//!
//! The lexer converts raw source bytes into a flat sequence of [`Token`]s.
//! It handles whitespace (including Unicode whitespace), single-line (`//`)
//! and block (`/* … */`) comments, punctuation, operators, string literals
//! (single- and double-quoted with escape sequences), integer and float
//! numeric literals, backtick-quoted identifiers, and keywords.
//!
//! Lexer errors are signalled by emitting a token of kind
//! [`SyntaxKind::ERROR`] rather than by returning a `Result`.

use crate::syntax::SyntaxKind;
use std::borrow::Cow;
use std::str::Chars;

/// A single lexer output token: a kind tag plus the byte length of the token.
///
/// The token does not carry the actual text; callers must slice the original
/// input string using the cumulative byte offsets of preceding tokens.
#[derive(Clone, Debug)]
pub struct Token {
    /// The syntactic kind of this token.
    pub kind: SyntaxKind,
    /// The byte length of the token in the source string.
    pub text_len: usize,
}

/// Incremental hand-written lexer for openCypher.
///
/// Call [`Lexer::advance`] repeatedly until it returns `None` to obtain all
/// tokens in the source. The lexer never emits `None` until the entire input
/// has been consumed.
#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    /// The original source text.
    input: &'a str,
    /// The remaining characters to consume.
    chars: Chars<'a>,
    /// The current byte offset within `input`.
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given `input` string.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars(),
            pos: 0,
        }
    }

    /// Peek at the next character without consuming it.
    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Peek at the character after the next without consuming anything.
    fn peek2(&self) -> Option<char> {
        let mut it = self.chars.clone();
        it.next();
        it.next()
    }

    /// Consume and return the next character, advancing `pos`.
    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.next();
        if let Some(c) = ch {
            self.pos += c.len_utf8();
        }
        ch
    }

    /// Return the current byte offset (start of the next token).
    fn start_pos(&self) -> usize {
        self.pos
    }

    /// Compute the byte length of a token that started at `start`.
    fn token_len(&self, start: usize) -> usize {
        self.pos - start
    }

    /// Construct a [`Token`] of `kind` spanning from `start` to the current position.
    fn make_token(&self, kind: SyntaxKind, start: usize) -> Token {
        Token {
            kind,
            text_len: self.token_len(start),
        }
    }

    /// Advance the lexer by one token and return it, or `None` at end of input.
    pub fn advance(&mut self) -> Option<Token> {
        let start = self.start_pos();
        let ch = self.peek()?;

        match ch {
            /* Whitespace & comments */
            ' ' | '\t' | '\r' | '\n' => {
                self.bump();
                while let Some(c) = self.peek() {
                    if is_whitespace(c) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                Some(self.make_token(SyntaxKind::WHITESPACE, start))
            }

            /* Comments */
            '/' if self.peek2() == Some('/') => {
                self.bump();
                self.bump();
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.bump();
                }
                Some(self.make_token(SyntaxKind::COMMENT, start))
            }
            '/' if self.peek2() == Some('*') => {
                self.bump();
                self.bump();
                while let Some(c) = self.peek() {
                    if c == '*' && self.peek2() == Some('/') {
                        self.bump();
                        self.bump();
                        break;
                    }
                    self.bump();
                }
                Some(self.make_token(SyntaxKind::COMMENT, start))
            }

            /* Punctuation */
            '(' => {
                self.bump();
                Some(self.make_token(SyntaxKind::L_PAREN, start))
            }
            ')' => {
                self.bump();
                Some(self.make_token(SyntaxKind::R_PAREN, start))
            }
            '{' => {
                self.bump();
                Some(self.make_token(SyntaxKind::L_BRACE, start))
            }
            '}' => {
                self.bump();
                Some(self.make_token(SyntaxKind::R_BRACE, start))
            }
            '[' => {
                self.bump();
                Some(self.make_token(SyntaxKind::L_BRACKET, start))
            }
            ']' => {
                self.bump();
                Some(self.make_token(SyntaxKind::R_BRACKET, start))
            }
            ',' => {
                self.bump();
                Some(self.make_token(SyntaxKind::COMMA, start))
            }
            ':' => {
                self.bump();
                Some(self.make_token(SyntaxKind::COLON, start))
            }
            '|' => {
                self.bump();
                Some(self.make_token(SyntaxKind::PIPE, start))
            }
            '$' => {
                self.bump();
                Some(self.make_token(SyntaxKind::DOLLAR, start))
            }
            '`' => {
                self.bump();
                while let Some(c) = self.peek() {
                    if c == '`' {
                        self.bump();
                        // Check if another backtick immediately follows (concatenated)
                        if self.peek() == Some('`') {
                            self.bump(); // consume start of next backtick run
                            continue; // continue reading inside it
                        }
                        break;
                    }
                    self.bump();
                }
                Some(self.make_token(SyntaxKind::ESCAPED_IDENT, start))
            }
            ';' => {
                self.bump();
                Some(self.make_token(SyntaxKind::SEMICOLON, start))
            }

            /* Operators — longer ones first */
            '<' if self.peek2() == Some('=') => {
                self.bump();
                self.bump();
                Some(self.make_token(SyntaxKind::LE, start))
            }
            '>' if self.peek2() == Some('=') => {
                self.bump();
                self.bump();
                Some(self.make_token(SyntaxKind::GE, start))
            }
            '<' if self.peek2() == Some('>') => {
                self.bump();
                self.bump();
                Some(self.make_token(SyntaxKind::NE, start))
            }
            '+' if self.peek2() == Some('=') => {
                self.bump();
                self.bump();
                Some(self.make_token(SyntaxKind::PLUSEQ, start))
            }
            '.' if self.peek2() == Some('.') => {
                self.bump();
                self.bump();
                Some(self.make_token(SyntaxKind::DOT_DOT, start))
            }
            '/' => {
                self.bump();
                Some(self.make_token(SyntaxKind::SLASH, start))
            }
            '<' => {
                self.bump();
                Some(self.make_token(SyntaxKind::LT, start))
            }
            '>' => {
                self.bump();
                Some(self.make_token(SyntaxKind::GT, start))
            }
            '=' => {
                self.bump();
                if self.peek() == Some('~') {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::TILDE_EQ, start));
                }
                Some(self.make_token(SyntaxKind::EQ, start))
            }
            '~' => {
                self.bump();
                Some(self.make_token(SyntaxKind::TILDE, start))
            }
            '+' => {
                self.bump();
                Some(self.make_token(SyntaxKind::PLUS, start))
            }
            '-' => {
                self.bump();
                Some(self.make_token(SyntaxKind::MINUS, start))
            }
            '*' => {
                self.bump();
                Some(self.make_token(SyntaxKind::STAR, start))
            }
            '%' => {
                self.bump();
                Some(self.make_token(SyntaxKind::PERCENT, start))
            }
            '^' => {
                self.bump();
                Some(self.make_token(SyntaxKind::POW, start))
            }
            '!' => {
                self.bump();
                Some(self.make_token(SyntaxKind::BANG, start))
            }
            '&' => {
                self.bump();
                Some(self.make_token(SyntaxKind::AMPERSAND, start))
            }
            '.' => {
                self.bump();
                Some(self.make_token(SyntaxKind::DOT, start))
            }

            /* Arrow heads (unicode variants for pattern matching) */
            '⟨' | '〈' | '﹤' | '＜' => {
                self.bump();
                Some(self.make_token(SyntaxKind::ARROW_LEFT, start))
            }
            '⟩' | '〉' | '﹥' | '＞' => {
                self.bump();
                Some(self.make_token(SyntaxKind::ARROW_RIGHT, start))
            }

            /* Dash variants */
            '\u{00AD}' | '‐' | '‑' | '‒' | '–' | '—' | '―' | '−' | '﹘' | '﹣' | '－' =>
            {
                self.bump();
                Some(self.make_token(SyntaxKind::DASH, start))
            }

            /* String literals */
            '"' | '\'' => {
                let quote = ch;
                self.bump();
                let mut terminated = false;
                while let Some(c) = self.peek() {
                    if c == quote {
                        self.bump();
                        terminated = true;
                        break;
                    }
                    if c == '\\' {
                        self.bump();
                        if let Some(escaped) = self.peek() {
                            self.bump();
                            match escaped {
                                'u' => {
                                    for _ in 0..4 {
                                        if let Some(h) = self.peek() {
                                            if h.is_ascii_hexdigit() {
                                                self.bump();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                                'U' => {
                                    for _ in 0..8 {
                                        if let Some(h) = self.peek() {
                                            if h.is_ascii_hexdigit() {
                                                self.bump();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }
                    if c == '\n' {
                        break;
                    }
                    self.bump();
                }
                let kind = if terminated {
                    SyntaxKind::STRING
                } else {
                    SyntaxKind::ERROR
                };
                Some(self.make_token(kind, start))
            }

            /* Numbers */
            '0'..='9' => {
                let kind = self.read_number(start);
                Some(self.make_token(kind, start))
            }

            /* Keywords and identifiers */
            _ if is_id_start(ch) => {
                let kind = self.read_ident_or_keyword(start);
                Some(self.make_token(kind, start))
            }

            _ => {
                self.bump();
                Some(self.make_token(SyntaxKind::ERROR, start))
            }
        }
    }

    /// Read a numeric literal starting at `start`.
    ///
    /// Handles hexadecimal (`0x…`), octal (`0o…`), decimal integers, and
    /// decimal floats (with optional `e`/`E` exponent). Returns
    /// [`SyntaxKind::INTEGER`] or [`SyntaxKind::FLOAT`].
    fn read_number(&mut self, start: usize) -> SyntaxKind {
        // Hex integer: 0x...
        if self.input[start..].starts_with("0x") || self.input[start..].starts_with("0X") {
            self.bump();
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_hexdigit() {
                    self.bump();
                } else {
                    break;
                }
            }
            return SyntaxKind::INTEGER;
        }

        // Octal integer: 0o... or 0O...
        if self.input[start..].starts_with("0o") || self.input[start..].starts_with("0O") {
            self.bump();
            self.bump();
            while let Some(c) = self.peek() {
                if matches!(c, '0'..='7') {
                    self.bump();
                } else {
                    break;
                }
            }
            return SyntaxKind::INTEGER;
        }

        let mut has_dot = false;
        let mut has_exp = false;

        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => {
                    self.bump();
                }
                Some('.') if !has_dot && !has_exp => {
                    // Check if next char after dot is a digit (float) or end/non-digit (property access)
                    if matches!(self.peek2(), Some(c) if c.is_ascii_digit()) {
                        has_dot = true;
                        self.bump();
                    } else {
                        break;
                    }
                }
                Some('e') | Some('E') if !has_exp => {
                    has_exp = true;
                    self.bump();
                    // Optional sign after exponent
                    if matches!(self.peek(), Some('-') | Some('+')) {
                        self.bump();
                    }
                }
                _ => break,
            }
        }

        if has_dot || has_exp {
            SyntaxKind::FLOAT
        } else {
            SyntaxKind::INTEGER
        }
    }

    /// Read an identifier or keyword starting at `start`.
    ///
    /// Consumes XID-continue characters after the first XID-start/underscore
    /// character, then looks up the result in the keyword table. Returns the
    /// matching keyword [`SyntaxKind`] or [`SyntaxKind::IDENT`].
    fn read_ident_or_keyword(&mut self, start: usize) -> SyntaxKind {
        self.bump();
        while let Some(c) = self.peek() {
            if is_id_continue(c) {
                self.bump();
            } else {
                break;
            }
        }

        let text = &self.input[start..self.pos];
        keyword_kind(text)
    }
}

/// Return `true` if `c` is a valid identifier start character.
///
/// Accepts Unicode XID-start characters plus underscore `_`.
fn is_id_start(c: char) -> bool {
    unicode_ident::is_xid_start(c) || c == '_'
}

/// Return `true` if `c` may appear after the first character of an identifier.
///
/// Accepts Unicode XID-continue characters.
fn is_id_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

/// Return `true` if `c` is considered whitespace by the openCypher spec.
///
/// Includes ASCII whitespace and a range of Unicode space separators.
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
        || matches!(
            c,
            '\u{00A0}'
                | '\u{1680}'
                | '\u{2000}'
                | '\u{2001}'
                | '\u{2002}'
                | '\u{2003}'
                | '\u{2004}'
                | '\u{2005}'
                | '\u{2006}'
                | '\u{2007}'
                | '\u{2008}'
                | '\u{2009}'
                | '\u{200A}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{2028}'
                | '\u{2029}'
        )
}

/// Map an identifier string to its keyword [`SyntaxKind`], if any.
///
/// The lookup is case-insensitive for ASCII identifiers. Non-ASCII
/// identifiers are compared verbatim (no uppercase mapping).
fn keyword_kind(s: &str) -> SyntaxKind {
    let upper: Cow<'_, str> = if s.is_ascii() {
        // Fast path for ASCII: use to_ascii_uppercase
        Cow::Owned(s.to_ascii_uppercase())
    } else {
        // For non-ASCII identifiers, just check against exact lowercase/uppercase forms
        Cow::Borrowed(s)
    };
    let key = upper.as_ref();
    match key {
        "ALL" => SyntaxKind::KW_ALL,
        "AND" => SyntaxKind::KW_AND,
        "ANY" => SyntaxKind::KW_ANY,
        "AS" => SyntaxKind::KW_AS,
        "ASC" => SyntaxKind::KW_ASC,
        "ASCENDING" => SyntaxKind::KW_ASCENDING,
        "BREAK" => SyntaxKind::KW_BREAK,
        "BY" => SyntaxKind::KW_BY,
        "CALL" => SyntaxKind::KW_CALL,
        "CASE" => SyntaxKind::KW_CASE,
        "CONTAINS" => SyntaxKind::KW_CONTAINS,
        "CONTINUE" => SyntaxKind::KW_CONTINUE,
        "CONSTRAINT" => SyntaxKind::KW_CONSTRAINT,
        "CONSTRAINTS" => SyntaxKind::KW_CONSTRAINTS,
        "CREATE" => SyntaxKind::KW_CREATE,
        "DATABASE" => SyntaxKind::KW_DATABASE,
        "DATABASES" => SyntaxKind::KW_DATABASES,
        "DELETE" => SyntaxKind::KW_DELETE,
        "DESC" => SyntaxKind::KW_DESC,
        "DESCENDING" => SyntaxKind::KW_DESCENDING,
        "DETACH" => SyntaxKind::KW_DETACH,
        "DISTINCT" => SyntaxKind::KW_DISTINCT,
        "DO" => SyntaxKind::KW_DO,
        "DROP" => SyntaxKind::KW_DROP,
        "ELSE" => SyntaxKind::KW_ELSE,
        "END" => SyntaxKind::KW_END,
        "ENDS" => SyntaxKind::KW_ENDS,
        "ERROR" => SyntaxKind::KW_ERROR,
        "EXISTS" => SyntaxKind::KW_EXISTS,
        "EXTRACT" => SyntaxKind::KW_EXTRACT,
        "FAIL" => SyntaxKind::KW_FAIL,
        "FILTER" => SyntaxKind::KW_FILTER,
        "FOR" => SyntaxKind::KW_FOR,
        "FOREACH" => SyntaxKind::KW_FOREACH,
        "EACH" => SyntaxKind::KW_EACH,
        "CONCURRENTLY" => SyntaxKind::KW_CONCURRENTLY,
        "FUNCTIONS" => SyntaxKind::KW_FUNCTIONS,
        "FULLTEXT" => SyntaxKind::KW_FULLTEXT,
        "GRAPH" => SyntaxKind::KW_GRAPH,
        "IF" => SyntaxKind::KW_IF,
        "IN" => SyntaxKind::KW_IN,
        "INDEX" => SyntaxKind::KW_INDEX,
        "INDEXES" => SyntaxKind::KW_INDEXES,
        "IS" => SyntaxKind::KW_IS,
        "KEY" => SyntaxKind::KW_KEY,
        "LIMIT" => SyntaxKind::KW_LIMIT,
        "LOOKUP" => SyntaxKind::KW_LOOKUP,
        "MANDATORY" => SyntaxKind::KW_MANDATORY,
        "MATCH" => SyntaxKind::KW_MATCH,
        "MERGE" => SyntaxKind::KW_MERGE,
        "NODE" => SyntaxKind::KW_NODE,
        "NONE" => SyntaxKind::KW_NONE,
        "NOT" => SyntaxKind::KW_NOT,
        "OF" => SyntaxKind::KW_OF,
        "ON" => SyntaxKind::KW_ON,
        "OPTIONAL" => SyntaxKind::KW_OPTIONAL,
        "OPTIONS" => SyntaxKind::KW_OPTIONS,
        "OR" => SyntaxKind::KW_OR,
        "ORDER" => SyntaxKind::KW_ORDER,
        "POINT" => SyntaxKind::KW_POINT,
        "PROCEDURES" => SyntaxKind::KW_PROCEDURES,
        "PROPERTY" => SyntaxKind::KW_PROPERTY,
        "RANGE" => SyntaxKind::KW_RANGE,
        "REDUCE" => SyntaxKind::KW_REDUCE,
        "REMOVE" => SyntaxKind::KW_REMOVE,
        "REQUIRE" => SyntaxKind::KW_REQUIRE,
        "RETURN" => SyntaxKind::KW_RETURN,
        "ROWS" => SyntaxKind::KW_ROWS,
        "SCALAR" => SyntaxKind::KW_SCALAR,
        "SET" => SyntaxKind::KW_SET,
        "SHOW" => SyntaxKind::KW_SHOW,
        "SINGLE" => SyntaxKind::KW_SINGLE,
        "SKIP" => SyntaxKind::KW_SKIP,
        "STARTS" => SyntaxKind::KW_STARTS,
        "TEXT" => SyntaxKind::KW_TEXT,
        "THEN" => SyntaxKind::KW_THEN,
        "TRANSACTIONS" => SyntaxKind::KW_TRANSACTIONS,
        "TYPE" => SyntaxKind::KW_TYPE,
        "TYPES" => SyntaxKind::KW_TYPES,
        "UNION" => SyntaxKind::KW_UNION,
        "UNIQUE" => SyntaxKind::KW_UNIQUE,
        "UNWIND" => SyntaxKind::KW_UNWIND,
        "USE" => SyntaxKind::KW_USE,
        "WHEN" => SyntaxKind::KW_WHEN,
        "WHERE" => SyntaxKind::KW_WHERE,
        "WITH" => SyntaxKind::KW_WITH,
        "XOR" => SyntaxKind::KW_XOR,
        "YIELD" => SyntaxKind::KW_YIELD,
        "HEADERS" => SyntaxKind::KW_HEADERS,
        "FROM" => SyntaxKind::KW_FROM,
        "LOAD" => SyntaxKind::KW_LOAD,
        "CSV" => SyntaxKind::KW_CSV,
        "FINISH" => SyntaxKind::KW_FINISH,
        "FIELDTERMINATOR" => SyntaxKind::KW_FIELDTERMINATOR,
        "COUNT" => SyntaxKind::KW_COUNT,
        "TRUE" => SyntaxKind::TRUE_KW,
        "FALSE" => SyntaxKind::FALSE_KW,
        "NULL" => SyntaxKind::NULL_KW,
        "ADD" => SyntaxKind::KW_ADD,
        "ACCESS" => SyntaxKind::KW_ACCESS,
        _ => SyntaxKind::IDENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn tokenize(input: &str) -> Vec<(SyntaxKind, &str)> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        while let Some(tok) = lexer.advance() {
            let text = &input[lexer.start_pos() - tok.text_len..lexer.start_pos()];
            tokens.push((tok.kind, text));
        }
        tokens
    }

    #[test]
    fn test_simple_match_return() {
        let tokens = tokenize("MATCH (n) RETURN n");
        let kinds: Vec<_> = tokens.iter().map(|(k, _)| *k).collect();
        check!(
            kinds
                == vec![
                    SyntaxKind::KW_MATCH,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::L_PAREN,
                    SyntaxKind::IDENT,
                    SyntaxKind::R_PAREN,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::KW_RETURN,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::IDENT,
                ]
        );
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("<= >= <> += ..");
        let kinds: Vec<_> = tokens.iter().map(|(k, _)| *k).collect();
        check!(
            kinds
                == vec![
                    SyntaxKind::LE,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::GE,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::NE,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::PLUSEQ,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::DOT_DOT,
                ]
        );
    }

    #[test]
    fn test_comments() {
        let tokens = tokenize("// line comment\nMATCH");
        let kinds: Vec<_> = tokens.iter().map(|(k, _)| *k).collect();
        check!(
            kinds
                == vec![
                    SyntaxKind::COMMENT,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::KW_MATCH,
                ]
        );
    }

    #[test]
    fn test_keywords_case_insensitive() {
        let tokens = tokenize("match where MATCH WHERE");
        let kinds: Vec<_> = tokens.iter().map(|(k, _)| *k).collect();
        check!(
            kinds
                == vec![
                    SyntaxKind::KW_MATCH,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::KW_WHERE,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::KW_MATCH,
                    SyntaxKind::WHITESPACE,
                    SyntaxKind::KW_WHERE,
                ]
        );
    }
}
