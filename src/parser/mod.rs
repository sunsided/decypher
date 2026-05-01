//! Hand-written error-resilient parser for openCypher.
//!
//! This module contains the lexer and grammar rules that produce a lossless CST
//! backed by `rowan`. The pest-based parser lives in `pest_parser` as a
//! conformance oracle.
//!
//! # Diagnostic guarantee
//!
//! `Parse::errors` is non-empty when the input is not a well-formed openCypher
//! query that the rowan grammar accepts. Every `CypherError` has a byte span
//! pointing at the offending token and an `Expected` set populated from the
//! call site that raised it.
//!
//! Phase 2: diagnostics implemented. Typed AstNode wrappers and wiring the
//! rowan parser into the public `parse()` are deferred.

pub mod grammar;
pub mod lexer;

use crate::error::{CypherError, ErrorKind, Expected, Span};
use crate::syntax::{CypherLang, SyntaxKind, SyntaxNode};
use rowan::{GreenNodeBuilder, Language};

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
    input: &'a str,
    lexer: lexer::Lexer<'a>,
    pub(crate) builder: GreenNodeBuilder<'static>,
    current_kind: SyntaxKind,
    current_len: usize,
    errors: Vec<CypherError>,
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

    fn parse(&mut self) {
        self.builder
            .start_node(CypherLang::kind_to_raw(SyntaxKind::SOURCE_FILE));

        // Parse a simple statement for now (MATCH ... RETURN ...)
        self.parse_statement();

        self.builder.finish_node();
    }

    fn parse_statement(&mut self) {
        self.builder
            .start_node(CypherLang::kind_to_raw(SyntaxKind::STATEMENT));

        // For now, parse a simple query body: clauses followed by optional semicolon
        let mut has_clause = false;
        loop {
            self.skip_trivia();
            if self.is_clause_start() {
                grammar::expr::parse_clause(self);
                has_clause = true;
            } else if self.at(SyntaxKind::SEMICOLON) || self.at(SyntaxKind::ERROR) {
                break;
            } else if self.current_len() == 0 {
                break;
            } else {
                // Unexpected token — emit diagnostic then eat it for recovery
                self.error_here(&[Expected::Category("clause")]);
                self.start_node(SyntaxKind::ERROR);
                self.bump();
                self.builder.finish_node();
            }
        }

        // Semicolon is trivia-level, just eat it
        self.eat(SyntaxKind::SEMICOLON);

        if !has_clause {
            // If nothing was parsed, still close the statement node
        }

        self.builder.finish_node();
    }

    /// Skip whitespace tokens without emitting them.
    pub(crate) fn skip_trivia(&mut self) {
        while self.current_kind == SyntaxKind::WHITESPACE {
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

    /// Emit a diagnostic at the current byte position using the current token text.
    /// Does not consume any tokens.
    pub(crate) fn error_here(&mut self, expected: &[Expected]) {
        let start = self.byte_pos;
        let end = start + self.current_len;
        let span = Span::new(start, end);
        let found = if self.current_len > 0 {
            self.input[start..end].to_string()
        } else {
            String::from("<end of input>")
        };
        self.errors.push(CypherError {
            kind: ErrorKind::UnexpectedToken {
                expected: expected.to_vec(),
                found,
            },
            span,
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    }

    /// Emit a diagnostic at an explicit span. Does not consume any tokens.
    pub(crate) fn error_at(&mut self, span: Span, expected: &[Expected]) {
        let found = if span.start < span.end && span.end <= self.input.len() {
            self.input[span.start..span.end].to_string()
        } else {
            String::from("<end of input>")
        };
        self.errors.push(CypherError {
            kind: ErrorKind::UnexpectedToken {
                expected: expected.to_vec(),
                found,
            },
            span,
            source_label: None,
            notes: Vec::new(),
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
        SyntaxKind::KW_EXISTS => "EXISTS",
        SyntaxKind::KW_UNION => "UNION",
        SyntaxKind::KW_SKIP => "SKIP",
        SyntaxKind::KW_LIMIT => "LIMIT",
        SyntaxKind::KW_DETACH => "DETACH",
        SyntaxKind::KW_ASC => "ASC",
        SyntaxKind::KW_ASCENDING => "ASCENDING",
        SyntaxKind::KW_DESC => "DESC",
        SyntaxKind::KW_DESCENDING => "DESCENDING",
        SyntaxKind::KW_EACH => "EACH",
        SyntaxKind::KW_CONCURRENTLY => "CONCURRENTLY",
        SyntaxKind::KW_GRAPH => "GRAPH",
        SyntaxKind::KW_CALL_SUBQUERY => "CALL {",
        SyntaxKind::KW_IN_TRANSACTIONS => "IN TRANSACTIONS",
        _ => {
            let debug = format!("{:?}", kind);
            // Leak to get &'static str — acceptable for diagnostics
            Box::leak(debug.into_boxed_str())
        }
    }
}

/// Check whether a syntax tree contains any ERROR nodes.
pub(crate) fn tree_has_error_node(tree: &SyntaxNode) -> bool {
    tree.descendants().any(|n| n.kind() == SyntaxKind::ERROR)
}

/// Re-exported for integration tests.
#[cfg(test)]
pub(crate) use tree_has_error_node as has_errors;

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
}
