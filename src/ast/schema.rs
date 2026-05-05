//! Schema command AST nodes for openCypher DDL statements.
//!
//! These types represent `CREATE INDEX`, `DROP INDEX`, `CREATE CONSTRAINT`,
//! `DROP CONSTRAINT`, `SHOW …`, and `USE` commands that manage the graph
//! schema rather than querying data.

use crate::ast::expr::{Expression, MapLiteral};
use crate::ast::names::{PropertyKeyName, SymbolicName, Variable};
use crate::error::Span;

/// A DDL schema command.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaCommand {
    /// `CREATE … INDEX …`
    CreateIndex(CreateIndex),
    /// `DROP INDEX …`
    DropIndex(DropIndex),
    /// `CREATE … CONSTRAINT …`
    CreateConstraint(CreateConstraint),
    /// `DROP CONSTRAINT …`
    DropConstraint(DropConstraint),
}

/// `CREATE [kind] INDEX [name] FOR (n:Label) ON (n.prop) [OPTIONS …]`
#[derive(Debug, Clone, PartialEq)]
pub struct CreateIndex {
    /// Optional index kind (`RANGE`, `TEXT`, `POINT`, `LOOKUP`, `FULLTEXT`).
    pub kind: Option<IndexKind>,
    /// `true` when `IF NOT EXISTS` was specified.
    pub if_not_exists: bool,
    /// Optional index name.
    pub name: Option<SymbolicName>,
    /// The label or relationship-type target.
    pub target: SymbolicName,
    /// Optional `OPTIONS` map.
    pub options: Option<MapLiteral>,
    /// Byte-offset span of the command.
    pub span: Span,
}

/// The type of index to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexKind {
    /// A range (B-tree) index — the default.
    Range,
    /// A full-text string index.
    Text,
    /// A spatial point index.
    Point,
    /// A token-lookup index.
    Lookup,
    /// A full-text index spanning multiple properties.
    Fulltext,
}

/// `DROP INDEX name [IF EXISTS]`
#[derive(Debug, Clone, PartialEq)]
pub struct DropIndex {
    /// `true` when `IF EXISTS` was specified (no error if the index is absent).
    pub if_exists: bool,
    /// The name of the index to drop.
    pub name: SymbolicName,
    /// Byte-offset span of the command.
    pub span: Span,
}

/// `CREATE CONSTRAINT [name] FOR (n:Label) REQUIRE … [IF NOT EXISTS]`
#[derive(Debug, Clone, PartialEq)]
pub struct CreateConstraint {
    /// Optional constraint name.
    pub name: Option<SymbolicName>,
    /// The node variable the constraint applies to.
    pub variable: Variable,
    /// The kind of constraint.
    pub kind: ConstraintKind,
    /// Byte-offset span of the command.
    pub span: Span,
}

/// The semantic kind of a `CREATE CONSTRAINT` command.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    /// `IS UNIQUE` — property uniqueness constraint.
    Unique,
    /// `IS NODE KEY` — composite node key constraint.
    NodeKey {
        /// The properties that together form the node key.
        properties: Vec<PropertyKeyName>,
    },
    /// `IS NOT NULL` — property existence constraint.
    NotNull,
    /// `IS :: Type` — property type constraint.
    PropertyType {
        /// The allowed property type names.
        types: Vec<SymbolicName>,
    },
}

/// `DROP CONSTRAINT name [IF EXISTS]`
#[derive(Debug, Clone, PartialEq)]
pub struct DropConstraint {
    /// `true` when `IF EXISTS` was specified.
    pub if_exists: bool,
    /// The name of the constraint to drop.
    pub name: SymbolicName,
    /// Byte-offset span of the command.
    pub span: Span,
}

/// `SHOW kind [YIELD …] [WHERE …] [RETURN …]`
#[derive(Debug, Clone, PartialEq)]
pub struct Show {
    /// What to show (indexes, constraints, functions, …).
    pub kind: ShowKind,
    /// Optional `YIELD` specification.
    pub yield_items: Option<ShowYieldSpec>,
    /// Optional `WHERE` filter.
    pub where_clause: Option<Expression>,
    /// Optional `RETURN` clause.
    pub return_clause: Option<ReturnBody>,
    /// Byte-offset span of the command.
    pub span: Span,
}

/// The category shown by a `SHOW` command.
#[derive(Debug, Clone, PartialEq)]
pub enum ShowKind {
    /// `SHOW INDEXES`
    Indexes,
    /// `SHOW CONSTRAINTS`
    Constraints,
    /// `SHOW FUNCTIONS`
    Functions,
    /// `SHOW PROCEDURES`
    Procedures,
    /// `SHOW DATABASES`
    Databases,
    /// `SHOW DATABASE name`
    Database(SymbolicName),
}

/// An inline `RETURN` body inside a `SHOW` command.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnBody {
    /// `true` when `RETURN DISTINCT`.
    pub distinct: bool,
    /// The projection items.
    pub items: Vec<crate::ast::clause::ProjectionItem>,
    /// Optional `ORDER BY`.
    pub order: Option<crate::ast::clause::Order>,
    /// Optional `SKIP n`.
    pub skip: Option<Expression>,
    /// Optional `LIMIT n`.
    pub limit: Option<Expression>,
}

/// `USE graph`
///
/// Selects the target graph for subsequent clauses.
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// The graph name or expression.
    pub graph: SymbolicName,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// The `YIELD` specification of a `SHOW` command.
#[derive(Debug, Clone, PartialEq)]
pub enum ShowYieldSpec {
    /// `YIELD *` — yield all available columns.
    Star {
        /// Byte-offset span of `YIELD *`.
        span: Span,
    },
    /// `YIELD col [AS alias], …`
    Items(Vec<ShowYieldItem>),
}

/// A single yielded column from a `SHOW` command.
#[derive(Debug, Clone, PartialEq)]
pub struct ShowYieldItem {
    /// The procedure/show result field name.
    pub procedure_field: SymbolicName,
    /// Optional `AS alias` rename.
    pub alias: Option<Variable>,
}
