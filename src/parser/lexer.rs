use crate::syntax::SyntaxKind;
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
                        if matches!(c, ' ' | '\t' | '\r' | '\n') {
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
                '\u{00AD}' | '‐' | '‑' | '‒' | '–' | '—' | '―' | '−' | '﹘' | '﹣' | '－' => {
                    self.bump();
                    return Some(self.make_token(SyntaxKind::DASH, start));
                }

                /* String literals */
                '"' | '\'' => {
                    let quote = ch;
                    self.bump();
                    while let Some(c) = self.peek() {
                        if c == quote {
                            self.bump();
                            break;
                        }
                        if c == '\\' {
                            self.bump();
                            if let Some(escaped) = self.peek() {
                                self.bump();
                                match escaped {
                                    'u' | 'U' => {
                                        for _ in 0..8 {
                                            if let Some(h) = self.peek() {
                                                if h.is_ascii_hexdigit() {
                                                    self.bump();
                                                } else {
                                                    break;
                                                }
                                            } else {
                                                break;
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
                    return Some(self.make_token(SyntaxKind::STRING, start));
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
                    if matches!(self.peek(), Some('-')) {
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
    c.is_ascii_alphabetic() || c == '_'
}

fn is_id_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn keyword_kind(s: &str) -> SyntaxKind {
    match s {
        "ALL" | "all" => SyntaxKind::KW_ALL,
        "AND" | "and" => SyntaxKind::KW_AND,
        "ANY" | "any" => SyntaxKind::KW_ANY,
        "AS" | "as" => SyntaxKind::KW_AS,
        "ASC" | "asc" => SyntaxKind::KW_ASC,
        "ASCENDING" | "ascending" => SyntaxKind::KW_ASCENDING,
        "BREAK" | "break" => SyntaxKind::KW_BREAK,
        "BY" | "by" => SyntaxKind::KW_BY,
        "CALL" | "call" => SyntaxKind::KW_CALL,
        "CASE" | "case" => SyntaxKind::KW_CASE,
        "CONTAINS" | "contains" => SyntaxKind::KW_CONTAINS,
        "CONTINUE" | "continue" => SyntaxKind::KW_CONTINUE,
        "CONSTRAINT" | "constraint" => SyntaxKind::KW_CONSTRAINT,
        "CONSTRAINTS" | "constraints" => SyntaxKind::KW_CONSTRAINTS,
        "CREATE" | "create" => SyntaxKind::KW_CREATE,
        "DATABASE" | "database" => SyntaxKind::KW_DATABASE,
        "DATABASES" | "databases" => SyntaxKind::KW_DATABASES,
        "DELETE" | "delete" => SyntaxKind::KW_DELETE,
        "DESC" | "desc" => SyntaxKind::KW_DESC,
        "DESCENDING" | "descending" => SyntaxKind::KW_DESCENDING,
        "DETACH" | "detach" => SyntaxKind::KW_DETACH,
        "DISTINCT" | "distinct" => SyntaxKind::KW_DISTINCT,
        "DO" | "do" => SyntaxKind::KW_DO,
        "DROP" | "drop" => SyntaxKind::KW_DROP,
        "ELSE" | "else" => SyntaxKind::KW_ELSE,
        "END" | "end" => SyntaxKind::KW_END,
        "ENDS" | "ends" => SyntaxKind::KW_ENDS,
        "ERROR" | "error" => SyntaxKind::KW_ERROR,
        "EXISTS" | "exists" => SyntaxKind::KW_EXISTS,
        "EXTRACT" | "extract" => SyntaxKind::KW_EXTRACT,
        "FAIL" | "fail" => SyntaxKind::KW_FAIL,
        "FILTER" | "filter" => SyntaxKind::KW_FILTER,
        "FOR" | "for" => SyntaxKind::KW_FOR,
        "FOREACH" | "foreach" => SyntaxKind::KW_FOREACH,
        "FUNCTIONS" | "functions" => SyntaxKind::KW_FUNCTIONS,
        "FULLTEXT" | "fulltext" => SyntaxKind::KW_FULLTEXT,
        "IF" | "if" => SyntaxKind::KW_IF,
        "IN" | "in" => SyntaxKind::KW_IN,
        "INDEX" | "index" => SyntaxKind::KW_INDEX,
        "INDEXES" | "indexes" => SyntaxKind::KW_INDEXES,
        "IS" | "is" => SyntaxKind::KW_IS,
        "KEY" | "key" => SyntaxKind::KW_KEY,
        "LIMIT" | "limit" => SyntaxKind::KW_LIMIT,
        "LOOKUP" | "lookup" => SyntaxKind::KW_LOOKUP,
        "MANDATORY" | "mandatory" => SyntaxKind::KW_MANDATORY,
        "MATCH" | "match" => SyntaxKind::KW_MATCH,
        "MERGE" | "merge" => SyntaxKind::KW_MERGE,
        "NODE" | "node" => SyntaxKind::KW_NODE,
        "NONE" | "none" => SyntaxKind::KW_NONE,
        "NOT" | "not" => SyntaxKind::KW_NOT,
        "OF" | "of" => SyntaxKind::KW_OF,
        "ON" | "on" => SyntaxKind::KW_ON,
        "OPTIONAL" | "optional" => SyntaxKind::KW_OPTIONAL,
        "OPTIONS" | "options" => SyntaxKind::KW_OPTIONS,
        "OR" | "or" => SyntaxKind::KW_OR,
        "ORDER" | "order" => SyntaxKind::KW_ORDER,
        "POINT" | "point" => SyntaxKind::KW_POINT,
        "PROCEDURES" | "procedures" => SyntaxKind::KW_PROCEDURES,
        "PROPERTY" | "property" => SyntaxKind::KW_PROPERTY,
        "RANGE" | "range" => SyntaxKind::KW_RANGE,
        "REMOVE" | "remove" => SyntaxKind::KW_REMOVE,
        "REQUIRE" | "require" => SyntaxKind::KW_REQUIRE,
        "RETURN" | "return" => SyntaxKind::KW_RETURN,
        "ROWS" | "rows" => SyntaxKind::KW_ROWS,
        "SCALAR" | "scalar" => SyntaxKind::KW_SCALAR,
        "SET" | "set" => SyntaxKind::KW_SET,
        "SHOW" | "show" => SyntaxKind::KW_SHOW,
        "SINGLE" | "single" => SyntaxKind::KW_SINGLE,
        "SKIP" | "skip" => SyntaxKind::KW_SKIP,
        "STARTS" | "starts" => SyntaxKind::KW_STARTS,
        "TEXT" | "text" => SyntaxKind::KW_TEXT,
        "THEN" | "then" => SyntaxKind::KW_THEN,
        "TRANSACTIONS" | "transactions" => SyntaxKind::KW_TRANSACTIONS,
        "TYPE" | "type" => SyntaxKind::KW_TYPE,
        "UNION" | "union" => SyntaxKind::KW_UNION,
        "UNIQUE" | "unique" => SyntaxKind::KW_UNIQUE,
        "UNWIND" | "unwind" => SyntaxKind::KW_UNWIND,
        "USE" | "use" => SyntaxKind::KW_USE,
        "WHEN" | "when" => SyntaxKind::KW_WHEN,
        "WHERE" | "where" => SyntaxKind::KW_WHERE,
        "WITH" | "with" => SyntaxKind::KW_WITH,
        "XOR" | "xor" => SyntaxKind::KW_XOR,
        "YIELD" | "yield" => SyntaxKind::KW_YIELD,
        "COUNT" | "count" => SyntaxKind::KW_COUNT,
        "TRUE" | "true" => SyntaxKind::TRUE_KW,
        "FALSE" | "false" => SyntaxKind::FALSE_KW,
        "NULL" | "null" => SyntaxKind::NULL_KW,
        "ADD" | "add" => SyntaxKind::KW_ADD,
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
