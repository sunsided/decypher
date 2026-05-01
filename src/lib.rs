//! open-cypher — parse openCypher queries into a typed AST.
#![allow(missing_docs)]

pub mod ast;
pub mod error;
mod parser;
mod pest_parser;
mod recover;
pub mod sema;
pub mod syntax;

/// Typed CST/AST wrappers over the lossless rowan CST.
///
/// This module provides rust-analyzer-style typed newtypes (`SourceFile`,
/// `MatchClause`, `NodePattern`, `Expression`, …) that wrap the raw
/// `SyntaxNode`/`SyntaxToken` produced by the rowan parser. Each wrapper
/// exposes accessor methods for semantically meaningful children instead of
/// requiring raw `SyntaxKind` matches.
///
/// # Stability
///
/// This API is **unstable** and may change before the rowan parser is wired
/// into the public [`parse`] function.
///
/// # Example
///
/// ```ignore
/// use open_cypher::cst::{parse, AstNode};
///
/// let result = parse("MATCH (n:Person) RETURN n.name");
/// let source = result.tree();
/// for stmt in source.statements() {
///     for clause in stmt.clauses() {
///         // …
///     }
/// }
/// ```
pub mod cst {
    pub use crate::parser::{parse, Parse};
    pub use crate::syntax::ast::clauses::*;
    pub use crate::syntax::ast::expressions::*;
    pub use crate::syntax::ast::patterns::*;
    pub use crate::syntax::ast::projection::*;
    pub use crate::syntax::ast::schema::*;
    pub use crate::syntax::ast::support::{
        child, child_token, child_tokens, children, AstChildren,
    };
    pub use crate::syntax::ast::tokens::*;
    pub use crate::syntax::ast::top_level::*;
    pub use crate::syntax::ast::{AstNode, AstToken};
}

#[cfg(feature = "low-level")]
pub mod low_level {
    pub use crate::pest_parser::{CypherParser, Rule};
    pub use pest::iterators::{Pair, Pairs};
}

pub use crate::ast::query::Query;
pub use crate::error::{CypherError, Diagnostics, ErrorKind, Expected, Note, Result, Span};
pub use crate::recover::{parse_with_options, ParseOptions};

use std::sync::Arc;

/// Parse a Cypher query string into a typed [`Query`] AST.
pub fn parse(input: &str) -> Result<Query> {
    parse_with_label(input, "query")
}

/// Parse a Cypher query string with a source label for diagnostics.
pub fn parse_with_label(input: &str, label: impl Into<Arc<str>>) -> Result<Query> {
    use crate::pest_parser::CypherParser;
    use pest::Parser;

    if input.trim().is_empty() {
        return Err(CypherError {
            kind: ErrorKind::EmptyInput,
            span: Span::new(0, 0),
            source_label: Some(label.into()),
            notes: Vec::new(),
            source: Some(Arc::from(input)),
        });
    }

    let source: Arc<str> = label.into();
    let original_source: Arc<str> = Arc::from(input);

    let pairs = CypherParser::parse(crate::pest_parser::Rule::Cypher, input).map_err(|e| {
        let mut err = error::translate_pest_error(e, original_source.clone());
        err.source_label = Some(source.clone());
        err
    })?;

    let top = pairs.into_iter().next().ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "empty parse result".into(),
        },
        span: Span::new(0, 0),
        source_label: Some(source.clone()),
        notes: Vec::new(),
        source: Some(original_source.clone()),
    })?;

    crate::ast::build::build_query(top).map_err(|mut e| {
        e.source_label = e.source_label.or_else(|| Some(source.clone()));
        if e.source.is_none() {
            e.source = Some(original_source.clone());
        }
        e
    })
}

/// Parse a Cypher query string, returning all diagnostics found.
pub fn parse_all(input: &str) -> (Option<Query>, Diagnostics) {
    parse_with_options(
        input,
        ParseOptions {
            recover: true,
            ..Default::default()
        },
    )
}

#[cfg(any(test, feature = "cst-parser"))]
pub fn parse_cst(input: &str) -> Result<Query> {
    use crate::syntax::ast::top_level::SourceFile;
    use crate::syntax::ast::AstNode;

    if input.trim().is_empty() {
        return Err(CypherError {
            kind: ErrorKind::EmptyInput,
            span: Span::new(0, 0),
            source_label: None,
            notes: Vec::new(),
            source: Some(Arc::from(input)),
        });
    }

    let parse = crate::parser::parse(input);
    let source = SourceFile::cast(parse.tree).ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "failed to cast to SourceFile".into(),
        },
        span: Span::new(0, 0),
        source_label: None,
        notes: Vec::new(),
        source: Some(Arc::from(input)),
    })?;
    crate::ast::build_cst::build_source_file(source)
}
