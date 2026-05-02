use crate::syntax::SyntaxKind;
use std::borrow::Cow;
use std::str::Chars;

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: SyntaxKind,
    pub text_len: usize,
}

#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    input: &'a str,
    chars: Chars<'a>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut it = self.chars.clone();
        it.next();
        it.next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.next();
        if let Some(c) = ch {
            self.pos += c.len_utf8();
        }
        ch
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn start_pos(&self) -> usize {
        self.pos
    }

    fn token_len(&self, start: usize) -> usize {
        self.pos - start
    }

    fn make_token(&self, kind: SyntaxKind, start: usize) -> Token {
        Token {
            kind,
            text_len: self.token_len(start),
        }
    }

    pub fn advance(&mut self) -> Option<Token> {
        loop {
            let start = self.start_pos();
            let ch = self.peek()?;

            let kind = match ch {
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
                    return Some(self.make_token(SyntaxKind::WHITESPACE, start));
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
                    return Some(self.make_token(SyntaxKind::COMMENT, start));
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
                    return Some(self.make_token(SyntaxKind::COMMENT, start));
                }

                /* Punctuation */
                '(' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::L_PAREN, start));
                }
                ')' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::R_PAREN, start));
                }
                '{' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::L_BRACE, start));
                }
                '}' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::R_BRACE, start));
                }
                '[' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::L_BRACKET, start));
                }
                ']' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::R_BRACKET, start));
                }
                ',' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::COMMA, start));
                }
                ':' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::COLON, start));
                }
                '|' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::PIPE, start));
                }
                '$' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::DOLLAR, start));
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
                    return Some(self.make_token(SyntaxKind::ESCAPED_IDENT, start));
                }
                ';' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::SEMICOLON, start));
                }

                /* Operators — longer ones first */
                '<' if self.peek2() == Some('=') => {
                    self.bump();
                    self.bump();
                    return Some(self.make_token(SyntaxKind::LE, start));
                }
                '>' if self.peek2() == Some('=') => {
                    self.bump();
                    self.bump();
                    return Some(self.make_token(SyntaxKind::GE, start));
                }
                '<' if self.peek2() == Some('>') => {
                    self.bump();
                    self.bump();
                    return Some(self.make_token(SyntaxKind::NE, start));
                }
                '+' if self.peek2() == Some('=') => {
                    self.bump();
                    self.bump();
                    return Some(self.make_token(SyntaxKind::PLUSEQ, start));
                }
                '.' if self.peek2() == Some('.') => {
                    self.bump();
                    self.bump();
                    return Some(self.make_token(SyntaxKind::DOT_DOT, start));
                }
                '/' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::SLASH, start));
                }
                '<' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::LT, start));
                }
                '>' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::GT, start));
                }
                '=' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::EQ, start));
                }
                '+' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::PLUS, start));
                }
                '-' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::MINUS, start));
                }
                '*' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::STAR, start));
                }
                '%' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::PERCENT, start));
                }
                '^' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::POW, start));
                }
                '.' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::DOT, start));
                }

                /* Arrow heads (unicode variants for pattern matching) */
                '⟨' | '〈' | '﹤' | '＜' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::ARROW_LEFT, start));
                }
                '⟩' | '〉' | '﹥' | '＞' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::ARROW_RIGHT, start));
                }

                /* Dash variants */
                '\u{00AD}' | '‐' | '‑' | '‒' | '–' | '—' | '―' | '−' | '﹘' | '﹣' | '－' =>
                {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::DASH, start));
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
                    return Some(self.make_token(kind, start));
                }

                /* Numbers */
                '0'..='9' => {
                    let kind = self.read_number(start);
                    return Some(self.make_token(kind, start));
                }

                /* Keywords and identifiers */
                _ if is_id_start(ch) => {
                    let kind = self.read_ident_or_keyword(start);
                    return Some(self.make_token(kind, start));
                }

                _ => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::ERROR, start));
                }
            };

            return Some(kind);
        }
    }

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

fn is_id_start(c: char) -> bool {
    unicode_ident::is_xid_start(c) || c == '_'
}

fn is_id_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

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
        assert_eq!(
            kinds,
            vec![
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
        assert_eq!(
            kinds,
            vec![
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
        assert_eq!(
            kinds,
            vec![
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
        assert_eq!(
            kinds,
            vec![
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
