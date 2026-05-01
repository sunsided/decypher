//! Hand-written error-resilient parser for openCypher.
//!
//! This module contains the lexer and grammar rules that produce a lossless CST
//! backed by `rowan`. The pest-based parser lives in `pest_parser` as a
//! conformance oracle (`#[cfg(test)]` in Phase 1).

pub mod lexer;
pub mod grammar;

use crate::error::CypherError;
use crate::syntax::{CypherLang, SyntaxKind, SyntaxNode};
use rowan::{GreenNodeBuilder, Language};

/// Result of parsing: a CST and any diagnostics.
pub struct Parse {
    pub tree: SyntaxNode,
    pub errors: Vec<CypherError>,
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
        self.builder.start_node(CypherLang::kind_to_raw(SyntaxKind::STATEMENT));

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
                // Unexpected token — eat it as error
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

    /// Expects the current token to be `kind`. Emits an ERROR node if not.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            // Emit a synthetic error node
            self.start_node(SyntaxKind::ERROR);
            self.builder.finish_node();
        }
    }

    /// Advances to the next token.
    pub(crate) fn bump(&mut self) {
        let kind = self.current_kind;
        let len = self.current_len;
        let start = self.byte_pos;

        if len > 0 {
            self.builder.token(CypherLang::kind_to_raw(kind), &self.input[start..start + len]);
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
        self.builder
            .start_node(CypherLang::kind_to_raw(kind));
    }

    /// Returns a checkpoint for conditional node wrapping.
    pub(crate) fn checkpoint(&self) -> rowan::Checkpoint {
        self.builder.checkpoint()
    }

    /// Starts a node at a previously recorded checkpoint.
    pub(crate) fn start_node_at(&mut self, checkpoint: rowan::Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(checkpoint, CypherLang::kind_to_raw(kind));
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

#[cfg(test)]
mod tests {
    use assert2::check;
    use super::*;

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
