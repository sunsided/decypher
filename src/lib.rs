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
pub use crate::error::{CypherError, Result, Span};

/// Parse a Cypher query string into a typed [`Query`] AST.
pub fn parse(input: &str) -> Result<Query> {
    use crate::parser::CypherParser;
    use pest::Parser;

    let pairs = CypherParser::parse(crate::parser::Rule::Cypher, input)?;
    let top = pairs.into_iter().next().ok_or_else(|| CypherError::Ast {
        message: "empty parse result".into(),
        span: Span::new(0, 0),
    })?;
    crate::ast::build::build_query(top)
}
