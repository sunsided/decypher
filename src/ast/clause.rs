//! Clause AST nodes for Cypher queries.
//!
//! Each struct in this module corresponds to one Cypher clause keyword.
//! Clauses are assembled into queries by the types in [`crate::ast::query`].

use crate::ast::expr::Expression;
use crate::ast::names::Variable;
use crate::ast::pattern::{Pattern, PatternPart};
use crate::error::Span;

/// `MATCH [OPTIONAL] pattern [WHERE predicate]`
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    /// `true` when the clause is `OPTIONAL MATCH`.
    pub optional: bool,
    /// The graph pattern to match.
    pub pattern: Pattern,
    /// Optional `WHERE` filter predicate.
    pub where_clause: Option<Expression>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// `CREATE pattern`
#[derive(Debug, Clone, PartialEq)]
pub struct Create {
    /// The graph pattern describing the nodes and relationships to create.
    pub pattern: Pattern,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// `MERGE pattern [ON MATCH SET …] [ON CREATE SET …]`
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    /// The pattern to merge (create if absent, match if present).
    pub pattern: PatternPart,
    /// Zero or more `ON MATCH` / `ON CREATE` set actions.
    pub actions: Vec<MergeAction>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// A single `ON MATCH SET …` or `ON CREATE SET …` action inside a `MERGE`.
#[derive(Debug, Clone, PartialEq)]
pub struct MergeAction {
    /// `true` for `ON MATCH`; `false` for `ON CREATE`.
    pub on_match: bool,
    /// The set items to apply when the action fires.
    pub set_items: Vec<SetItem>,
    /// Byte-offset span of this action.
    pub span: Span,
}

/// `[DETACH] DELETE targets`
#[derive(Debug, Clone, PartialEq)]
pub struct Delete {
    /// `true` when `DETACH DELETE` (also removes all incident relationships).
    pub detach: bool,
    /// The expressions identifying the nodes or relationships to delete.
    pub targets: Vec<Expression>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// `SET item, …`
#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    /// The set items to apply.
    pub items: Vec<SetItem>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// A single item inside a `SET` clause.
#[derive(Debug, Clone, PartialEq)]
pub enum SetItem {
    /// `property = value` or `property += value`
    Property {
        /// The property expression (e.g. `n.name`).
        property: Expression,
        /// The value to assign.
        value: Expression,
        /// The assignment operator (`=` or `+=`).
        operator: SetOperator,
    },
    /// `variable = value` or `variable += value`
    Variable {
        /// The variable being assigned.
        variable: Variable,
        /// The value to assign.
        value: Expression,
        /// The assignment operator (`=` or `+=`).
        operator: SetOperator,
    },
    /// `variable:Label1:Label2 …` — add labels to a node.
    Labels {
        /// The node variable to relabel.
        variable: Variable,
        /// The labels to add.
        labels: Vec<crate::ast::names::SymbolicName>,
    },
}

/// The operator used in a [`SetItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperator {
    /// `=` — overwrite the property or variable.
    Assign,
    /// `+=` — merge (add) properties from the RHS map.
    Add,
}

/// `REMOVE item, …`
#[derive(Debug, Clone, PartialEq)]
pub struct Remove {
    /// The items to remove (labels or properties).
    pub items: Vec<RemoveItem>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// A single item inside a `REMOVE` clause.
#[derive(Debug, Clone, PartialEq)]
pub enum RemoveItem {
    /// `variable:Label1:Label2` — remove labels from a node.
    Labels {
        /// The node variable.
        variable: Variable,
        /// The labels to remove.
        labels: Vec<crate::ast::names::SymbolicName>,
    },
    /// `expression` — remove a property (e.g. `n.name`).
    Property(Expression),
}

/// `WITH [DISTINCT] [*] items [ORDER BY …] [SKIP …] [LIMIT …] [WHERE …]`
///
/// Pipes results from one query part to the next, optionally transforming and
/// filtering the visible scope.
#[derive(Debug, Clone, PartialEq)]
pub struct With {
    /// `true` when `WITH DISTINCT`.
    pub distinct: bool,
    /// `true` when `WITH *` (include all current bindings).
    pub star: bool,
    /// Explicitly projected items.
    pub items: Vec<ProjectionItem>,
    /// Optional `ORDER BY` ordering.
    pub order: Option<Order>,
    /// Optional `SKIP n` expression.
    pub skip: Option<Expression>,
    /// Optional `LIMIT n` expression.
    pub limit: Option<Expression>,
    /// Optional `WHERE` filter applied after projection.
    pub where_clause: Option<Expression>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// `RETURN [DISTINCT] [*] items [ORDER BY …] [SKIP …] [LIMIT …]`
#[derive(Debug, Clone, PartialEq)]
pub struct Return {
    /// `true` when `RETURN DISTINCT`.
    pub distinct: bool,
    /// `true` when `RETURN *` (return all current bindings).
    pub star: bool,
    /// Explicitly returned items.
    pub items: Vec<ProjectionItem>,
    /// Optional `ORDER BY` ordering.
    pub order: Option<Order>,
    /// Optional `SKIP n` expression.
    pub skip: Option<Expression>,
    /// Optional `LIMIT n` expression.
    pub limit: Option<Expression>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// A single item in a `RETURN` or `WITH` projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionItem {
    /// The expression to project.
    pub expression: Expression,
    /// Optional `AS alias` rename.
    pub alias: Option<Variable>,
}

/// `ORDER BY item, …`
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    /// The ordered list of sort criteria.
    pub items: Vec<SortItem>,
}

/// A single sort criterion inside `ORDER BY`.
#[derive(Debug, Clone, PartialEq)]
pub struct SortItem {
    /// The expression to sort by.
    pub expression: Expression,
    /// Explicit direction, or `None` for the default (ascending).
    pub direction: Option<SortDirection>,
}

/// Sort direction for an `ORDER BY` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// `ASC` / `ASCENDING`
    Ascending,
    /// `DESC` / `DESCENDING`
    Descending,
}

/// `UNWIND expression AS variable`
///
/// Expands a list expression into individual rows, binding each element to
/// `variable`.
#[derive(Debug, Clone, PartialEq)]
pub struct Unwind {
    /// The list expression to expand.
    pub expression: Expression,
    /// The variable bound to each element.
    pub variable: Variable,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// `FOREACH (variable IN list | update, …)`
///
/// Iterates over a list expression and applies updating clauses for each
/// element.
#[derive(Debug, Clone, PartialEq)]
pub struct Foreach {
    /// The loop variable bound to each list element.
    pub variable: Variable,
    /// The list expression to iterate.
    pub list: Expression,
    /// The updating clauses applied for each element.
    pub updates: Vec<ForeachUpdate>,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// An updating clause that may appear inside a [`Foreach`] body.
#[derive(Debug, Clone, PartialEq)]
pub enum ForeachUpdate {
    /// `CREATE` inside a foreach.
    Create(Create),
    /// `MERGE` inside a foreach.
    Merge(Merge),
    /// `DELETE` inside a foreach.
    Delete(Delete),
    /// `SET` inside a foreach.
    Set(Set),
    /// `REMOVE` inside a foreach.
    Remove(Remove),
    /// Nested `FOREACH` inside a foreach.
    Foreach(Foreach),
}

/// `LOAD CSV [WITH HEADERS] FROM source AS variable`
///
/// Reads a CSV file and binds each row (as a map or list) to `variable`.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadCsv {
    /// `true` when `WITH HEADERS` (rows are bound as string→string maps).
    pub with_headers: bool,
    /// The source URL expression.
    pub source: Expression,
    /// The variable bound to each CSV row.
    pub variable: Variable,
    /// Byte-offset span of the clause.
    pub span: Span,
}

/// `FINISH`
///
/// GQL extension: terminates a query without returning any results.
#[derive(Debug, Clone, PartialEq)]
pub struct Finish {
    /// Byte-offset span of the clause.
    pub span: Span,
}
