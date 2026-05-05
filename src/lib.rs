//! A Rust library for parsing Cypher queries into a typed AST.
//!
//! # Overview
//!
//! This crate exposes three levels of representation for a Cypher query
//! string:
//!
//! 1. **CST** – a lossless concrete syntax tree built on [rowan], available
//!    via [`parse_cst`] and the [`cst`] module.
//! 2. **AST** – a typed, high-level abstract syntax tree, available via
//!    [`parse`] and the [`ast`] module.
//! 3. **HIR** – a lowered, scope-resolved high-level intermediate
//!    representation, available via [`analyze`] and the [`hir`] module.
//!
//! # Quick start
//!
//! ```
//! use cypher::parse;
//!
//! let query = parse("MATCH (n:Person) RETURN n.name").unwrap();
//! println!("{} statement(s)", query.statements.len());
//! ```
//!
//! For multi-error recovery use [`parse_all`]:
//!
//! ```
//! use cypher::parse_all;
//!
//! let (query, diagnostics) = parse_all("RETURN;");
//! assert!(query.is_none());
//! assert!(!diagnostics.is_empty());
//! ```
//!
//! [rowan]: https://docs.rs/rowan

pub mod ast;
pub mod error;
pub mod hir;
mod parser;
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
/// This API is **unstable** and may change as the CST matures.
///
/// # Example
///
/// ```ignore
/// use cypher::cst::{parse, AstNode};
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
    pub use crate::parser::{Parse, parse};
    pub use crate::syntax::ast::clauses::*;
    pub use crate::syntax::ast::expressions::*;
    pub use crate::syntax::ast::patterns::*;
    pub use crate::syntax::ast::projection::*;
    pub use crate::syntax::ast::schema::*;
    pub use crate::syntax::ast::support::{
        AstChildren, child, child_token, child_tokens, children,
    };
    pub use crate::syntax::ast::tokens::*;
    pub use crate::syntax::ast::top_level::*;
    pub use crate::syntax::ast::{AstNode, AstToken};
}

pub use crate::ast::query::Query;
pub use crate::error::{CypherError, Diagnostics, ErrorKind, Expected, Note, Result, Span};
pub use crate::parser::Parse;
pub use crate::recover::{ParseOptions, parse_with_options};

use std::sync::Arc;

impl TryFrom<&str> for Query {
    type Error = CypherError;

    fn try_from(input: &str) -> Result<Self> {
        parse(input)
    }
}

/// Parse a Cypher query string into a typed [`Query`] AST.
///
/// Returns `Ok(Query)` on success. On the first parse error the function
/// returns `Err(CypherError)` with position information and diagnostic notes.
/// For collecting *all* errors in one pass, use [`parse_all`].
///
/// # Errors
///
/// Returns [`CypherError`] when the input is empty, syntactically invalid, or
/// contains unsupported grammar constructs.
///
/// # Example
///
/// ```
/// use cypher::parse;
///
/// let query = parse("MATCH (n:Person) RETURN n.name").unwrap();
/// assert_eq!(query.statements.len(), 1);
/// ```
pub fn parse(input: &str) -> Result<Query> {
    parse_with_label(input, "query")
}

/// Parse a Cypher query string with an explicit source label used in diagnostics.
///
/// The `label` is stored in any [`CypherError`] produced, allowing consumers to
/// display the originating file or source name alongside error messages.
///
/// # Errors
///
/// Returns [`CypherError`] on any parse or AST-construction error.
///
/// # Example
///
/// ```
/// use cypher::parse_with_label;
///
/// let result = parse_with_label("RETURN 1", "my_script.cypher");
/// assert!(result.is_ok());
///
/// let result = parse_with_label("RETURN;", "my_script.cypher");
/// let err = result.unwrap_err();
/// assert_eq!(err.source_label(), Some("my_script.cypher"));
/// ```
pub fn parse_with_label(input: &str, label: impl Into<Arc<str>>) -> Result<Query> {
    use crate::syntax::ast::AstNode;
    use crate::syntax::ast::top_level::SourceFile;

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

    let parse = crate::parser::parse(input);

    if !parse.errors.is_empty() {
        let mut err = parse.errors.into_iter().next().unwrap();
        err.source_label = Some(source.clone());
        if err.source.is_none() {
            err.source = Some(original_source.clone());
        }
        return Err(err);
    }

    let source_file = SourceFile::cast(parse.tree).ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "failed to cast to SourceFile".into(),
        },
        span: Span::new(0, 0),
        source_label: Some(source.clone()),
        notes: Vec::new(),
        source: Some(original_source.clone()),
    })?;
    crate::ast::build_cst::build_source_file(source_file).map_err(|mut e| {
        e.source_label = e.source_label.or_else(|| Some(source.clone()));
        if e.source.is_none() {
            e.source = Some(original_source.clone());
        }
        e
    })
}

/// Parse a Cypher query string in error-recovery mode, returning all
/// diagnostics discovered during parsing.
///
/// Unlike [`parse`], this function does not stop at the first error; it
/// attempts to resynchronise at statement boundaries and continue. The
/// returned `Option<Query>` is `Some` only when a statement was successfully
/// parsed *after* the last resynchronisation point. A valid statement that
/// appears before a later syntax error does not guarantee `Some`: the
/// implementation keeps only the successfully parsed suffix after recovery,
/// not any valid prefix.
///
/// # Example
///
/// ```
/// use cypher::parse_all;
///
/// let (query, diagnostics) = parse_all("RETURN;");
/// assert!(query.is_none());
/// assert!(!diagnostics.is_empty());
/// ```
pub fn parse_all(input: &str) -> (Option<Query>, Diagnostics) {
    parse_with_options(
        input,
        ParseOptions {
            recover: true,
            ..Default::default()
        },
    )
}

/// Parse a Cypher query string into the lossless rowan CST.
///
/// This returns the raw [`Parse`] result containing the concrete syntax tree
/// and any parser diagnostics. For the typed AST, use [`parse`] instead.
///
/// # Example
///
/// ```
/// use cypher::parse_cst;
///
/// let cst = parse_cst("MATCH (n) RETURN n");
/// assert!(cst.errors.is_empty());
/// ```
pub fn parse_cst(input: &str) -> Parse {
    crate::parser::parse(input)
}

/// Parse and lower a Cypher query into a [`hir::HirQuery`].
///
/// This is a convenience function that chains [`parse`] and
/// [`hir::lower::lower`]. It performs syntax parsing, AST construction, and
/// HIR lowering (scope resolution, graph pattern normalisation) in a single
/// call. Returns the first [`CypherError`] on failure.
///
/// The input can be either a `&str` (which will be parsed via [`parse`]) or an
/// already-parsed [`Query`] (which is used as-is, skipping the parse step).
///
/// # Errors
///
/// Returns the first error encountered during parsing or HIR lowering.
///
/// # Example: from a string
///
/// ```
/// let hir = cypher::analyze("MATCH (n:Person) RETURN n.name").unwrap();
/// assert!(!hir.parts.is_empty());
/// ```
///
/// # Example: from a previously parsed AST
///
/// ```
/// let query = cypher::parse("MATCH (n:Person) RETURN n.name").unwrap();
/// let hir = cypher::analyze(query).unwrap();
/// assert!(!hir.parts.is_empty());
/// ```
pub fn analyze<T>(input: T) -> Result<hir::HirQuery>
where
    T: TryInto<Query>,
    CypherError: From<T::Error>,
{
    let query = input.try_into().map_err(CypherError::from)?;
    hir::lower::lower(&query).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .next()
            .unwrap_or_else(|| CypherError {
                kind: ErrorKind::Internal {
                    message: "unknown HIR lowering error".into(),
                },
                span: Span::new(0, 0),
                source_label: None,
                notes: Vec::new(),
                source: None,
            })
    })
}
