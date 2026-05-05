//! Procedure call AST nodes for Cypher queries.
//!
//! Cypher supports two forms of procedure invocation:
//! - [`StandaloneCall`]: a top-level `CALL proc()` statement.
//! - [`InQueryCall`]: an in-query `CALL proc() YIELD …` clause embedded
//!   inside a larger query.

use crate::ast::expr::{Expression, FunctionInvocation};
use crate::ast::names::Variable;
use crate::error::Span;

/// A top-level `CALL procedure() [YIELD …]` statement.
///
/// Appears as a standalone statement, not embedded in a data query.
#[derive(Debug, Clone, PartialEq)]
pub struct StandaloneCall {
    /// The procedure being called.
    pub call: ProcedureInvocation,
    /// Optional `YIELD` specification (or `YIELD *`).
    pub yield_items: Option<YieldSpec>,
    /// Byte-offset span of the statement.
    pub span: Span,
}

/// An in-query `CALL procedure() YIELD …` clause.
///
/// Embedded inside a reading part of a query to call a procedure and
/// project its result columns into the query scope.
#[derive(Debug, Clone, PartialEq)]
pub struct InQueryCall {
    /// The procedure being called.
    pub call: ProcedureInvocation,
    /// Optional `YIELD` items (columns to project into scope).
    pub yield_items: Option<YieldItems>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// A procedure invocation: the name and arguments.
///
/// Reuses [`FunctionInvocation`] for the qualified name and argument list.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureInvocation {
    /// The qualified procedure name and arguments.
    pub name: FunctionInvocation,
    /// Byte-offset span of the invocation.
    pub span: Span,
}

/// The `YIELD` specification of a [`StandaloneCall`].
#[derive(Debug, Clone, PartialEq)]
pub enum YieldSpec {
    /// `YIELD *` — yield all result columns.
    Star {
        /// Byte-offset span of `YIELD *`.
        span: Span,
    },
    /// `YIELD col [AS alias], …` — yield selected columns.
    Items(YieldItems),
}

/// A list of explicitly yielded procedure result columns with an optional
/// `WHERE` filter.
#[derive(Debug, Clone, PartialEq)]
pub struct YieldItems {
    /// The yielded column items.
    pub items: Vec<YieldItem>,
    /// Optional `WHERE` filter applied to the yielded rows.
    pub where_clause: Option<Expression>,
}

/// A single yielded column from a procedure call.
#[derive(Debug, Clone, PartialEq)]
pub struct YieldItem {
    /// The name of the procedure result field.
    pub procedure_field: crate::ast::names::SymbolicName,
    /// Optional `AS alias` rename into the query scope.
    pub alias: Option<Variable>,
}
