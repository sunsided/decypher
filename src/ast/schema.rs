use crate::ast::expr::{Expression, MapLiteral};
use crate::ast::names::{PropertyKeyName, SymbolicName, Variable};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaCommand {
    CreateIndex(CreateIndex),
    DropIndex(DropIndex),
    CreateConstraint(CreateConstraint),
    DropConstraint(DropConstraint),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateIndex {
    pub kind: Option<IndexKind>,
    pub if_not_exists: bool,
    pub name: Option<SymbolicName>,
    pub target: SymbolicName,
    pub options: Option<MapLiteral>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexKind {
    Range,
    Text,
    Point,
    Lookup,
    Fulltext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropIndex {
    pub if_exists: bool,
    pub name: SymbolicName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateConstraint {
    pub name: Option<SymbolicName>,
    pub variable: Variable,
    pub kind: ConstraintKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    Unique,
    NodeKey { properties: Vec<PropertyKeyName> },
    NotNull,
    PropertyType { types: Vec<SymbolicName> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropConstraint {
    pub if_exists: bool,
    pub name: SymbolicName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Show {
    pub kind: ShowKind,
    pub yield_items: Option<ShowYieldSpec>,
    pub where_clause: Option<Expression>,
    pub return_clause: Option<ReturnBody>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShowKind {
    Indexes,
    Constraints,
    Functions,
    Procedures,
    Databases,
    Database(SymbolicName),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnBody {
    pub distinct: bool,
    pub items: Vec<crate::ast::clause::ProjectionItem>,
    pub order: Option<crate::ast::clause::Order>,
    pub skip: Option<Expression>,
    pub limit: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub graph: SymbolicName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShowYieldSpec {
    Star { span: Span },
    Items(Vec<ShowYieldItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShowYieldItem {
    pub procedure_field: SymbolicName,
    pub alias: Option<Variable>,
}
