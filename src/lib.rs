//! open-cypher — parse openCypher queries into a typed AST.
#![allow(missing_docs)]

pub mod ast;
pub mod error;
mod parser;

#[cfg(feature = "low-level")]
pub mod low_level {
    pub use crate::parser::{CypherParser, Rule};
    pub use pest::iterators::{Pair, Pairs};
}

pub use crate::ast::query::Query;
pub use crate::error::{CypherError, Diagnostics, ErrorKind, Expected, Note, Result, Span};

use std::sync::Arc;

/// Parse a Cypher query string into a typed [`Query`] AST.
pub fn parse(input: &str) -> Result<Query> {
    parse_with_label(input, "query")
}

/// Parse a Cypher query string with a source label for diagnostics.
pub fn parse_with_label(input: &str, label: impl Into<Arc<str>>) -> Result<Query> {
    use crate::parser::CypherParser;
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

    let pairs = CypherParser::parse(crate::parser::Rule::Cypher, input).map_err(|e| {
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
///
/// Currently returns at most one error, but the signature is stable to allow
/// multi-error recovery in future versions.
pub fn parse_all(input: &str) -> (Option<Query>, Diagnostics) {
    match parse(input) {
        Ok(query) => (Some(query), Diagnostics { errors: Vec::new() }),
        Err(err) => (None, Diagnostics { errors: vec![err] }),
    }
}
