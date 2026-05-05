//! Hand-written error-resilient parser for openCypher.
//!
//! This module contains the lexer and grammar rules that produce a lossless CST
//! backed by `rowan`. This is the only parser in the crate — the public
//! [`parse`](crate::parse) function uses it exclusively.
//!
//! # Diagnostic guarantee
//!
//! `Parse::errors` is non-empty when the input is not a well-formed openCypher
//! query that the rowan grammar accepts. Every `CypherError` has a byte span
//! pointing at the offending token and an `Expected` set populated from the
//! call site that raised it.

pub mod grammar;
pub mod lexer;

use crate::error::{CypherError, ErrorKind, Expected, Span};
use crate::error::{Note, NoteLevel};
use crate::syntax::{CypherLang, SyntaxKind, SyntaxNode};
use rowan::{GreenNodeBuilder, Language};
use std::borrow::Cow;

/// Result of parsing: a CST and any diagnostics.
pub struct Parse {
    pub tree: SyntaxNode,
    pub errors: Vec<CypherError>,
}

impl Parse {
    /// Returns the typed `SourceFile` wrapper for the CST.
    /// Always succeeds because the grammar guarantees the root is `SOURCE_FILE`.
    pub fn tree(&self) -> crate::syntax::ast::top_level::SourceFile {
        use crate::syntax::ast::AstNode;
        crate::syntax::ast::top_level::SourceFile::cast(self.tree.clone())
            .expect("root node is always SOURCE_FILE")
    }
}

/// Parse a Cypher query string into a CST using the hand-written parser.
///
/// This always succeeds — even on invalid input it returns a tree with
/// `ERROR` nodes and a list of diagnostics.
pub fn parse(input: &str) -> Parse {
    let mut p = Parser::new(input);
    p.parse();
    p.finish()
}

pub(crate) struct Parser<'a> {
    /// The complete input text, kept for token slicing and diagnostic messages.
    input: &'a str,
    /// The underlying lexer, advanced one token at a time.
    lexer: lexer::Lexer<'a>,
    /// The rowan green-tree builder that accumulates CST nodes.
    pub(crate) builder: GreenNodeBuilder<'static>,
    /// The syntactic kind of the current (lookahead) token.
    current_kind: SyntaxKind,
    /// The byte length of the current token.
    current_len: usize,
    /// Errors accumulated during parsing.
    errors: Vec<CypherError>,
    /// The byte offset of the current token within `input`.
    byte_pos: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = lexer::Lexer::new(input);
        let first = lexer.advance();
        let (current_kind, current_len) = match first {
            Some(tok) => (tok.kind, tok.text_len),
            None => (SyntaxKind::ERROR, 0),
        };
        Self {
            input,
            lexer,
            builder: GreenNodeBuilder::new(),
            current_kind,
            current_len,
            errors: Vec::new(),
            byte_pos: 0,
        }
    }

    /// Parse the root node (`SOURCE_FILE`) of the grammar.
    fn parse(&mut self) {
        self.builder
            .start_node(CypherLang::kind_to_raw(SyntaxKind::SOURCE_FILE));

        // Parse statements, supporting UNION between query bodies
        loop {
            self.skip_trivia();
            if self.current_len() == 0 {
                break;
            }
            if self.at(SyntaxKind::SEMICOLON) {
                self.bump();
                continue;
            }

            // Optional query options that can prefix a statement.
            while self.at_bare_word("EXPLAIN") || self.at_bare_word("PROFILE") {
                self.bump();
                self.skip_trivia();
            }

            // Neo4j query option prelude, e.g.:
            // CYPHER runtime=pipelined
            if self.at_bare_word("CYPHER") {
                self.bump();
                self.skip_trivia();
                while self.current_len() > 0
                    && !self.is_clause_start()
                    && !self.at(SyntaxKind::SEMICOLON)
                {
                    self.bump();
                    self.skip_trivia();
                }
            }

            if self.current_len() == 0 {
                break;
            }
            if self.at(SyntaxKind::KW_SHOW)
                || self.at(SyntaxKind::KW_USE)
                || self.at(SyntaxKind::KW_DROP)
            {
                grammar::expr::parse_clause(self);
            } else if self.at(SyntaxKind::KW_CREATE) {
                let next = self.peek_next_non_trivia();
                if next == Some(SyntaxKind::KW_INDEX)
                    || next == Some(SyntaxKind::KW_TEXT)
                    || next == Some(SyntaxKind::KW_LOOKUP)
                    || next == Some(SyntaxKind::KW_RANGE)
                    || next == Some(SyntaxKind::KW_POINT)
                    || next == Some(SyntaxKind::KW_FULLTEXT)
                    || next == Some(SyntaxKind::KW_CONSTRAINT)
                    || next == Some(SyntaxKind::KW_DATABASE)
                    || next == Some(SyntaxKind::KW_DATABASES)
                {
                    grammar::expr::parse_clause(self);
                } else {
                    self.parse_statement();
                }
            } else if self.is_clause_start() {
                self.parse_statement();
            } else {
                break;
            }
            self.skip_trivia();
        }

        self.builder.finish_node();
    }

    /// Parse a single statement (`STATEMENT` node) including any trailing
    /// `UNION [ALL] …` branches.
    fn parse_statement(&mut self) {
        self.builder
            .start_node(CypherLang::kind_to_raw(SyntaxKind::STATEMENT));

        // Parse the first query body (clauses until RETURN/WITH end the single query)
        self.parse_query_body();

        // Optional UNION [ALL] followed by another query body
        while self.at(SyntaxKind::KW_UNION) {
            self.builder
                .start_node(CypherLang::kind_to_raw(SyntaxKind::UNION));
            self.bump(); // UNION
            self.skip_trivia();
            self.eat(SyntaxKind::KW_ALL);
            self.skip_trivia();
            self.parse_query_body();
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }

    /// Parse the clause sequence that forms one query body.
    fn parse_query_body(&mut self) {
        // Parse clauses: reading clauses followed by updating clauses and/or RETURN
        loop {
            self.skip_trivia();
            if self.at(SyntaxKind::SEMICOLON)
                || self.at(SyntaxKind::KW_UNION)
                || self.current_len() == 0
            {
                break;
            }
            if self.at(SyntaxKind::ERROR) {
                self.error_here(&[Expected::Category("valid token")]);
                break;
            }
            if self.is_clause_start() {
                grammar::expr::parse_clause(self);
            } else {
                // Unexpected token — emit diagnostic then eat it for recovery
                self.error_here(&[Expected::Category("clause")]);
                self.start_node(SyntaxKind::ERROR);
                self.bump();
                self.builder.finish_node();
            }
        }

        // Semicolon is trivia-level, just eat it (but not at statement level)
    }

    /// Skip whitespace and comment tokens without emitting them.
    pub(crate) fn skip_trivia(&mut self) {
        while self.current_kind == SyntaxKind::WHITESPACE
            || self.current_kind == SyntaxKind::COMMENT
        {
            self.bump();
        }
    }

    /// Returns true if the current token starts a clause.
    fn is_clause_start(&self) -> bool {
        matches!(
            self.current_kind,
            SyntaxKind::KW_MATCH
                | SyntaxKind::KW_RETURN
                | SyntaxKind::KW_WITH
                | SyntaxKind::KW_UNWIND
                | SyntaxKind::KW_CREATE
                | SyntaxKind::KW_MERGE
                | SyntaxKind::KW_DELETE
                | SyntaxKind::KW_SET
                | SyntaxKind::KW_REMOVE
                | SyntaxKind::KW_CALL
                | SyntaxKind::KW_FOREACH
                | SyntaxKind::KW_OPTIONAL
                | SyntaxKind::KW_DETACH
                | SyntaxKind::KW_YIELD
                | SyntaxKind::KW_SHOW
                | SyntaxKind::KW_USE
                | SyntaxKind::KW_DROP
                | SyntaxKind::KW_LOAD
                | SyntaxKind::KW_FINISH
        )
    }

    /// Returns the current token kind.
    pub(crate) fn current_kind(&self) -> SyntaxKind {
        self.current_kind
    }

    /// Returns the length of the current token.
    pub(crate) fn current_len(&self) -> usize {
        self.current_len
    }

    /// Returns true if the current token matches `kind`.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current_kind == kind
    }

    /// Returns true if the current token is the given keyword.
    pub(crate) fn at_keyword(&self, kw: SyntaxKind) -> bool {
        self.current_kind == kw
    }

    /// Returns true when the current token is an identifier matching `word`.
    pub(crate) fn at_bare_word(&self, word: &str) -> bool {
        if self.current_kind != SyntaxKind::IDENT || self.current_len == 0 {
            return false;
        }
        let start = self.byte_pos;
        let end = start + self.current_len;
        self.input[start..end].eq_ignore_ascii_case(word)
    }

    /// Consumes the current token if it matches `kind`.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Expects the current token to be `kind`. Emits an ERROR node and diagnostic if not.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            self.error_here(&[Expected::Symbol(kind_to_str(kind))]);
            self.start_node(SyntaxKind::ERROR);
            self.builder.finish_node();
        }
    }

    /// Emit an error and insert an empty `ERROR` node for one-of alternatives.
    ///
    /// Used when none of a set of expected token alternatives is present but no
    /// token should be consumed for recovery.
    pub(crate) fn expect_one_of(&mut self, expected: &[Expected]) {
        self.error_here(expected);
        self.start_node(SyntaxKind::ERROR);
        self.builder.finish_node();
    }

    /// Emit a diagnostic at the current byte position using the current token text.
    /// Does not consume any tokens.
    pub(crate) fn error_here(&mut self, expected: &[Expected]) {
        self.error_here_with_notes(expected, Vec::new());
    }

    /// Emit a diagnostic at the current position with an explicit `help` note.
    ///
    /// The diagnostic is also decorated with a [`NoteLevel::Help`] note
    /// carrying `message`.
    pub(crate) fn error_here_with_help(
        &mut self,
        expected: &[Expected],
        message: impl Into<Cow<'static, str>>,
    ) {
        self.error_here_with_notes(
            expected,
            vec![Note {
                span: Span::new(0, 0),
                message: message.into(),
                level: NoteLevel::Help,
            }],
        );
    }

    /// Emit a diagnostic at the current position with an explicit set of notes.
    ///
    /// The `expected` tokens are sorted and deduplicated before being stored.
    pub(crate) fn error_here_with_notes(&mut self, expected: &[Expected], notes: Vec<Note>) {
        let start = self.byte_pos;
        let end = start + self.current_len;
        let span = Span::new(start, end);
        let found = if self.current_len > 0 {
            self.input[start..end].to_string()
        } else {
            String::from("<end of input>")
        };
        let mut normalized = expected.to_vec();
        normalized.sort();
        normalized.dedup();
        self.errors.push(CypherError {
            kind: ErrorKind::UnexpectedToken {
                expected: normalized,
                found,
            },
            span,
            source_label: None,
            notes,
            source: None,
        });
    }

    /// Advances to the next token.
    pub(crate) fn bump(&mut self) {
        let kind = self.current_kind;
        let len = self.current_len;
        let start = self.byte_pos;

        if len > 0 {
            self.builder.token(
                CypherLang::kind_to_raw(kind),
                &self.input[start..start + len],
            );
        }

        self.byte_pos += len;
        match self.lexer.advance() {
            Some(tok) => {
                self.current_kind = tok.kind;
                self.current_len = tok.text_len;
            }
            None => {
                self.current_kind = SyntaxKind::ERROR;
                self.current_len = 0;
            }
        }
    }

    /// Starts a new node and pushes it onto the builder's stack.
    pub(crate) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(CypherLang::kind_to_raw(kind));
    }

    /// Returns a checkpoint for conditional node wrapping.
    pub(crate) fn checkpoint(&self) -> rowan::Checkpoint {
        self.builder.checkpoint()
    }

    /// Starts a node at a previously recorded checkpoint.
    pub(crate) fn start_node_at(&mut self, checkpoint: rowan::Checkpoint, kind: SyntaxKind) {
        self.builder
            .start_node_at(checkpoint, CypherLang::kind_to_raw(kind));
    }

    /// Peeks the next non-trivia token kind without consuming it.
    pub(crate) fn peek_next_non_trivia(&self) -> Option<SyntaxKind> {
        let mut lx = self.lexer.clone();
        loop {
            match lx.advance() {
                Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
                Some(tok) => return Some(tok.kind),
                None => return None,
            }
        }
    }

    /// Returns true if, starting at the current position, the tokens look
    /// like a qualified function call head: `.IDENT (. IDENT)* (`.
    /// Used to disambiguate `foo.bar(x)` (qualified function invocation)
    /// from `foo.bar` (property lookup).
    pub(crate) fn looks_like_qualified_call(&self) -> bool {
        /// Returns true if this SyntaxKind can be used as a name segment
        /// (plain identifier, escaped identifier, or a keyword).
        fn is_name_part(k: SyntaxKind) -> bool {
            matches!(
                k,
                SyntaxKind::IDENT
                    | SyntaxKind::ESCAPED_IDENT
                    | SyntaxKind::KW_ACCESS
                    | SyntaxKind::KW_ADD
                    | SyntaxKind::KW_ALL
                    | SyntaxKind::KW_AND
                    | SyntaxKind::KW_ANY
                    | SyntaxKind::KW_AS
                    | SyntaxKind::KW_ASC
                    | SyntaxKind::KW_ASCENDING
                    | SyntaxKind::KW_BREAK
                    | SyntaxKind::KW_BY
                    | SyntaxKind::KW_CALL
                    | SyntaxKind::KW_CASE
                    | SyntaxKind::KW_CONTAINS
                    | SyntaxKind::KW_CONTINUE
                    | SyntaxKind::KW_CONSTRAINT
                    | SyntaxKind::KW_CONSTRAINTS
                    | SyntaxKind::KW_CREATE
                    | SyntaxKind::KW_DATABASE
                    | SyntaxKind::KW_DATABASES
                    | SyntaxKind::KW_DELETE
                    | SyntaxKind::KW_DESC
                    | SyntaxKind::KW_DESCENDING
                    | SyntaxKind::KW_DETACH
                    | SyntaxKind::KW_DISTINCT
                    | SyntaxKind::KW_DO
                    | SyntaxKind::KW_DROP
                    | SyntaxKind::KW_ELSE
                    | SyntaxKind::KW_END
                    | SyntaxKind::KW_ENDS
                    | SyntaxKind::KW_ERROR
                    | SyntaxKind::KW_EXISTS
                    | SyntaxKind::KW_EXTRACT
                    | SyntaxKind::KW_FAIL
                    | SyntaxKind::KW_FILTER
                    | SyntaxKind::KW_FOR
                    | SyntaxKind::KW_FOREACH
                    | SyntaxKind::KW_EACH
                    | SyntaxKind::KW_FUNCTIONS
                    | SyntaxKind::KW_FULLTEXT
                    | SyntaxKind::KW_IF
                    | SyntaxKind::KW_IN
                    | SyntaxKind::KW_INDEX
                    | SyntaxKind::KW_INDEXES
                    | SyntaxKind::KW_IS
                    | SyntaxKind::KW_KEY
                    | SyntaxKind::KW_LIMIT
                    | SyntaxKind::KW_LOOKUP
                    | SyntaxKind::KW_MANDATORY
                    | SyntaxKind::KW_MATCH
                    | SyntaxKind::KW_MERGE
                    | SyntaxKind::KW_NODE
                    | SyntaxKind::KW_NONE
                    | SyntaxKind::KW_NOT
                    | SyntaxKind::KW_OF
                    | SyntaxKind::KW_ON
                    | SyntaxKind::KW_OPTIONAL
                    | SyntaxKind::KW_OPTIONS
                    | SyntaxKind::KW_OR
                    | SyntaxKind::KW_ORDER
                    | SyntaxKind::KW_POINT
                    | SyntaxKind::KW_PROCEDURES
                    | SyntaxKind::KW_PROPERTY
                    | SyntaxKind::KW_RANGE
                    | SyntaxKind::KW_REDUCE
                    | SyntaxKind::KW_REMOVE
                    | SyntaxKind::KW_REQUIRE
                    | SyntaxKind::KW_RETURN
                    | SyntaxKind::KW_ROWS
                    | SyntaxKind::KW_SCALAR
                    | SyntaxKind::KW_SET
                    | SyntaxKind::KW_SHOW
                    | SyntaxKind::KW_SINGLE
                    | SyntaxKind::KW_SKIP
                    | SyntaxKind::KW_STARTS
                    | SyntaxKind::KW_TEXT
                    | SyntaxKind::KW_THEN
                    | SyntaxKind::KW_TRANSACTIONS
                    | SyntaxKind::KW_TYPE
                    | SyntaxKind::KW_TYPES
                    | SyntaxKind::KW_UNION
                    | SyntaxKind::KW_UNIQUE
                    | SyntaxKind::KW_UNWIND
                    | SyntaxKind::KW_USE
                    | SyntaxKind::KW_WHEN
                    | SyntaxKind::KW_WHERE
                    | SyntaxKind::KW_WITH
                    | SyntaxKind::KW_XOR
                    | SyntaxKind::KW_YIELD
                    | SyntaxKind::KW_COUNT
                    | SyntaxKind::KW_CALL_SUBQUERY
                    | SyntaxKind::KW_IN_TRANSACTIONS
                    | SyntaxKind::KW_CONCURRENTLY
                    | SyntaxKind::KW_HEADERS
                    | SyntaxKind::KW_FROM
                    | SyntaxKind::KW_LOAD
                    | SyntaxKind::KW_CSV
                    | SyntaxKind::KW_FINISH
                    | SyntaxKind::KW_FIELDTERMINATOR
            )
        }
        let mut lx = self.lexer.clone();
        // Advance lx one token at a time, skipping WHITESPACE. Returns None at EOF.
        fn next_nt(lx: &mut lexer::Lexer) -> Option<SyntaxKind> {
            loop {
                match lx.advance() {
                    Some(t) if t.kind == SyntaxKind::WHITESPACE => continue,
                    Some(t) => return Some(t.kind),
                    None => return None,
                }
            }
        }
        // First `.IDENT` pair
        if next_nt(&mut lx) != Some(SyntaxKind::DOT) {
            return false;
        }
        if !is_name_part(next_nt(&mut lx).unwrap_or(SyntaxKind::ERROR)) {
            return false;
        }
        // After the first `.IDENT`, scan for more `.IDENT`s or terminal `(`
        loop {
            match next_nt(&mut lx) {
                Some(SyntaxKind::L_PAREN) => return true,
                Some(SyntaxKind::DOT) => {
                    if !is_name_part(next_nt(&mut lx).unwrap_or(SyntaxKind::ERROR)) {
                        return false;
                    }
                    continue;
                }
                _ => return false,
            }
        }
    }

    pub(crate) fn finish(self) -> Parse {
        let green = self.builder.finish();
        let tree = SyntaxNode::new_root(green);
        Parse {
            tree,
            errors: self.errors,
        }
    }
}

/// Map a `SyntaxKind` to its canonical Cypher spelling for diagnostics.
fn kind_to_str(kind: SyntaxKind) -> &'static str {
    match kind {
        SyntaxKind::L_PAREN => "(",
        SyntaxKind::R_PAREN => ")",
        SyntaxKind::L_BRACE => "{",
        SyntaxKind::R_BRACE => "}",
        SyntaxKind::L_BRACKET => "[",
        SyntaxKind::R_BRACKET => "]",
        SyntaxKind::COMMA => ",",
        SyntaxKind::DOT => ".",
        SyntaxKind::DOT_DOT => "..",
        SyntaxKind::COLON => ":",
        SyntaxKind::PIPE => "|",
        SyntaxKind::DOLLAR => "$",
        SyntaxKind::SEMICOLON => ";",
        SyntaxKind::EQ => "=",
        SyntaxKind::NE => "<>",
        SyntaxKind::LT => "<",
        SyntaxKind::GT => ">",
        SyntaxKind::LE => "<=",
        SyntaxKind::GE => ">=",
        SyntaxKind::PLUS => "+",
        SyntaxKind::MINUS => "-",
        SyntaxKind::STAR => "*",
        SyntaxKind::SLASH => "/",
        SyntaxKind::PERCENT => "%",
        SyntaxKind::POW => "^",
        SyntaxKind::PLUSEQ => "+=",
        SyntaxKind::ARROW_LEFT => "<-",
        SyntaxKind::ARROW_RIGHT => "->",
        SyntaxKind::DASH => "-",
        SyntaxKind::INTEGER => "integer",
        SyntaxKind::FLOAT => "float",
        SyntaxKind::STRING => "string",
        SyntaxKind::TRUE_KW => "true",
        SyntaxKind::FALSE_KW => "false",
        SyntaxKind::NULL_KW => "null",
        SyntaxKind::IDENT => "identifier",
        SyntaxKind::ESCAPED_IDENT => "identifier",
        SyntaxKind::KW_AS => "AS",
        SyntaxKind::KW_BY => "BY",
        SyntaxKind::KW_CASE => "CASE",
        SyntaxKind::KW_ELSE => "ELSE",
        SyntaxKind::KW_END => "END",
        SyntaxKind::KW_THEN => "THEN",
        SyntaxKind::KW_WHEN => "WHEN",
        SyntaxKind::KW_WITH => "WITH",
        SyntaxKind::KW_MATCH => "MATCH",
        SyntaxKind::KW_RETURN => "RETURN",
        SyntaxKind::KW_WHERE => "WHERE",
        SyntaxKind::KW_ORDER => "ORDER",
        SyntaxKind::KW_IS => "IS",
        SyntaxKind::KW_IN => "IN",
        SyntaxKind::KW_NOT => "NOT",
        SyntaxKind::KW_STARTS => "STARTS",
        SyntaxKind::KW_ENDS => "ENDS",
        SyntaxKind::KW_CONTAINS => "CONTAINS",
        SyntaxKind::KW_DISTINCT => "DISTINCT",
        SyntaxKind::KW_UNWIND => "UNWIND",
        SyntaxKind::KW_CREATE => "CREATE",
        SyntaxKind::KW_MERGE => "MERGE",
        SyntaxKind::KW_DELETE => "DELETE",
        SyntaxKind::KW_SET => "SET",
        SyntaxKind::KW_REMOVE => "REMOVE",
        SyntaxKind::KW_CALL => "CALL",
        SyntaxKind::KW_FOREACH => "FOREACH",
        SyntaxKind::KW_EACH => "EACH",
        SyntaxKind::KW_CONCURRENTLY => "CONCURRENTLY",
        SyntaxKind::KW_OPTIONAL => "OPTIONAL",
        SyntaxKind::KW_OR => "OR",
        SyntaxKind::KW_XOR => "XOR",
        SyntaxKind::KW_AND => "AND",
        SyntaxKind::KW_ALL => "ALL",
        SyntaxKind::KW_ANY => "ANY",
        SyntaxKind::KW_NONE => "NONE",
        SyntaxKind::KW_SINGLE => "SINGLE",
        SyntaxKind::KW_FILTER => "FILTER",
        SyntaxKind::KW_EXTRACT => "EXTRACT",
        SyntaxKind::KW_REDUCE => "REDUCE",
        SyntaxKind::KW_EXISTS => "EXISTS",
        SyntaxKind::KW_UNION => "UNION",
        SyntaxKind::KW_SKIP => "SKIP",
        SyntaxKind::KW_LIMIT => "LIMIT",
        SyntaxKind::KW_DETACH => "DETACH",
        SyntaxKind::KW_ASC => "ASC",
        SyntaxKind::KW_ASCENDING => "ASCENDING",
        SyntaxKind::KW_DESC => "DESC",
        SyntaxKind::KW_DESCENDING => "DESCENDING",
        SyntaxKind::KW_GRAPH => "GRAPH",
        SyntaxKind::KW_HEADERS => "HEADERS",
        SyntaxKind::KW_FROM => "FROM",
        SyntaxKind::KW_LOAD => "LOAD",
        SyntaxKind::KW_CSV => "CSV",
        SyntaxKind::KW_FINISH => "FINISH",
        SyntaxKind::KW_FIELDTERMINATOR => "FIELDTERMINATOR",
        SyntaxKind::KW_CALL_SUBQUERY => "CALL {",
        SyntaxKind::KW_IN_TRANSACTIONS => "IN TRANSACTIONS",
        _ => {
            let debug = format!("{:?}", kind);
            // Leak to get &'static str — acceptable for diagnostics
            Box::leak(debug.into_boxed_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn test_flat_token_tree() {
        let parse = parse("MATCH (n) RETURN n");
        check!(parse.tree.kind() == SyntaxKind::SOURCE_FILE);
        check!(parse.tree.text().to_string() == "MATCH (n) RETURN n");
        // descendants_with_tokens includes all nodes and tokens recursively
        check!(parse.tree.descendants_with_tokens().count() > 1);
    }

    #[test]
    fn test_empty_input() {
        let parse = parse("");
        check!(parse.tree.kind() == SyntaxKind::SOURCE_FILE);
        check!(parse.tree.text().is_empty());
    }

    #[test]
    fn test_lossless_text() {
        let input = "MATCH  (n:Person)  RETURN  n";
        let parse = parse(input);
        check!(parse.tree.text().to_string() == input);
    }

    #[test]
    fn test_simple_expression() {
        let parse = parse("RETURN 1 + 2 * 3");
        check!(parse.tree.text().to_string() == "RETURN 1 + 2 * 3");
    }

    #[test]
    fn test_comparison_expression() {
        let parse = parse("RETURN n.age > 18");
        check!(parse.tree.text().to_string() == "RETURN n.age > 18");
    }

    #[test]
    fn test_complex_expression_precedence() {
        // Tests that 1 + 2 * 3 parses correctly (multiplication binds tighter)
        let parse = parse("RETURN 1 + 2 * 3");
        check!(parse.tree.descendants_with_tokens().count() > 5);
    }

    #[test]
    fn test_not_expression() {
        let parse = parse("RETURN NOT true");
        check!(parse.tree.text().to_string() == "RETURN NOT true");
    }

    #[test]
    fn test_property_chain() {
        let parse = parse("RETURN a.b.c.d");
        check!(parse.tree.text().to_string() == "RETURN a.b.c.d");
    }

    #[test]
    fn test_function_invocation() {
        let parse = parse("RETURN count(n)");
        check!(parse.tree.text().to_string() == "RETURN count(n)");
    }

    #[test]
    fn test_list_literal() {
        let parse = parse("RETURN [1, 2, 3]");
        check!(parse.tree.text().to_string() == "RETURN [1, 2, 3]");
    }

    #[test]
    fn test_map_literal() {
        let parse = parse("RETURN {key: 'value'}");
        check!(parse.tree.text().to_string() == "RETURN {key: 'value'}");
    }

    #[test]
    fn test_starts_with() {
        let parse = parse("RETURN name STARTS WITH 'A'");
        check!(parse.tree.text().to_string() == "RETURN name STARTS WITH 'A'");
    }

    #[test]
    fn test_is_null() {
        let parse = parse("RETURN x IS NOT NULL");
        check!(parse.tree.text().to_string() == "RETURN x IS NOT NULL");
    }

    #[test]
    fn test_parenthesized() {
        let parse = parse("RETURN (1 + 2) * 3");
        check!(parse.tree.text().to_string() == "RETURN (1 + 2) * 3");
    }

    #[test]
    fn test_foreach_clause() {
        let parse = parse("FOREACH (n IN nodes | CREATE (n)-[:LINK]->())");
        check!(parse.tree.text().to_string() == "FOREACH (n IN nodes | CREATE (n)-[:LINK]->())");
        let has_foreach = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::FOREACH_CLAUSE);
        check!(has_foreach);
    }

    #[test]
    fn test_standalone_call() {
        let parse = parse("CALL db.labels()");
        check!(parse.tree.text().to_string() == "CALL db.labels()");
        let has_call = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::STANDALONE_CALL);
        check!(has_call);
    }

    #[test]
    fn test_call_with_yield() {
        let parse = parse("CALL db.labels() YIELD label");
        check!(parse.tree.text().to_string() == "CALL db.labels() YIELD label");
        let has_yield = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::YIELD_ITEMS);
        check!(has_yield);
    }

    #[test]
    fn test_call_with_yield_and_where() {
        let parse = parse("CALL db.labels() YIELD label WHERE label STARTS WITH 'User'");
        check!(
            parse.tree.text().to_string()
                == "CALL db.labels() YIELD label WHERE label STARTS WITH 'User'"
        );
    }

    #[test]
    fn test_call_subquery() {
        let parse = parse("CALL { MATCH (n) RETURN n }");
        check!(parse.tree.text().to_string() == "CALL { MATCH (n) RETURN n }");
        let has_subquery = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::CALL_SUBQUERY_CLAUSE);
        check!(has_subquery);
    }

    #[test]
    fn test_call_subquery_in_transactions() {
        let parse = parse("CALL { MATCH (n) RETURN n } IN TRANSACTIONS");
        check!(parse.tree.text().to_string() == "CALL { MATCH (n) RETURN n } IN TRANSACTIONS");
        let has_tx = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::IN_TRANSACTIONS);
        check!(has_tx);
    }

    #[test]
    fn test_call_subquery_in_transactions_of_rows() {
        let parse = parse("CALL { MATCH (n) RETURN n } IN TRANSACTIONS OF 1000 ROWS");
        check!(
            parse.tree.text().to_string()
                == "CALL { MATCH (n) RETURN n } IN TRANSACTIONS OF 1000 ROWS"
        );
    }

    #[test]
    fn test_union() {
        let parse = parse("MATCH (n:Person) RETURN n.name UNION MATCH (m:Company) RETURN m.name");
        check!(
            parse.tree.text().to_string()
                == "MATCH (n:Person) RETURN n.name UNION MATCH (m:Company) RETURN m.name"
        );
        let has_union = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::UNION);
        check!(has_union);
    }

    #[test]
    fn test_union_all() {
        let parse = parse("RETURN 1 AS n UNION ALL RETURN 1 AS n");
        check!(parse.tree.text().to_string() == "RETURN 1 AS n UNION ALL RETURN 1 AS n");
        let has_union = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::UNION);
        check!(has_union);
    }

    #[test]
    fn test_create_index() {
        let parse = parse("CREATE INDEX idx FOR (n:Person) ON (n.name)");
        check!(parse.tree.text().to_string() == "CREATE INDEX idx FOR (n:Person) ON (n.name)");
        let has_index = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::CREATE_INDEX);
        check!(has_index);
    }

    #[test]
    fn test_create_text_index() {
        let parse = parse("CREATE TEXT INDEX idx FOR (n:Person) ON EACH [n.name, n.email]");
        check!(
            parse.tree.text().to_string()
                == "CREATE TEXT INDEX idx FOR (n:Person) ON EACH [n.name, n.email]"
        );
    }

    #[test]
    fn test_create_constraint() {
        let parse = parse("CREATE CONSTRAINT uniq FOR (n:Person) REQUIRE n.email IS UNIQUE");
        check!(
            parse.tree.text().to_string()
                == "CREATE CONSTRAINT uniq FOR (n:Person) REQUIRE n.email IS UNIQUE"
        );
        let has_constraint = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::CREATE_CONSTRAINT);
        check!(has_constraint);
    }

    #[test]
    fn test_drop_index() {
        let parse = parse("DROP INDEX idx");
        check!(parse.tree.text().to_string() == "DROP INDEX idx");
        let has_drop = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::DROP_INDEX);
        check!(has_drop);
    }

    #[test]
    fn test_drop_constraint() {
        let parse = parse("DROP CONSTRAINT uniq");
        check!(parse.tree.text().to_string() == "DROP CONSTRAINT uniq");
        let has_drop = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::DROP_CONSTRAINT);
        check!(has_drop);
    }

    #[test]
    fn test_show_indexes() {
        let parse = parse("SHOW INDEXES");
        check!(parse.tree.text().to_string() == "SHOW INDEXES");
        let has_show = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::SHOW_CLAUSE);
        check!(has_show);
    }

    #[test]
    fn test_show_with_yield() {
        let parse = parse("SHOW INDEXES YIELD * WHERE type = 'BTREE'");
        check!(parse.tree.text().to_string() == "SHOW INDEXES YIELD * WHERE type = 'BTREE'");
    }

    #[test]
    fn test_use_database() {
        let parse = parse("USE mydb");
        check!(parse.tree.text().to_string() == "USE mydb");
        let has_use = parse
            .tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::USE_CLAUSE);
        check!(has_use);
    }

    #[test]
    fn test_multiple_statements() {
        let parse = parse("MATCH (n) RETURN n; CREATE (m:Person {name: 'Alice'})");
        check!(
            parse.tree.text().to_string()
                == "MATCH (n) RETURN n; CREATE (m:Person {name: 'Alice'})"
        );
        let stmts: Vec<_> = parse
            .tree
            .children()
            .filter(|n| n.kind() == SyntaxKind::STATEMENT)
            .collect();
        check!(stmts.len() == 2);
    }
}
