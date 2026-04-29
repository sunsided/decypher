use crate::ast::expr::Expression;
use crate::ast::names::Variable;
use crate::ast::pattern::{Pattern, PatternPart};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub optional: bool,
    pub pattern: Pattern,
    pub where_clause: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Create {
    pub pattern: Pattern,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub pattern: PatternPart,
    pub actions: Vec<MergeAction>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeAction {
    pub on_match: bool,
    pub set_items: Vec<SetItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Delete {
    pub detach: bool,
    pub targets: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    pub items: Vec<SetItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetItem {
    Property {
        property: Expression,
        value: Expression,
    },
    Variable {
        variable: Variable,
        value: Expression,
        operator: SetOperator,
    },
    Labels {
        variable: Variable,
        labels: Vec<crate::ast::names::SymbolicName>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperator {
    Assign,
    Add,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Remove {
    pub items: Vec<RemoveItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoveItem {
    Labels {
        variable: Variable,
        labels: Vec<crate::ast::names::SymbolicName>,
    },
    Property(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct With {
    pub distinct: bool,
    pub items: Vec<ProjectionItem>,
    pub order: Option<Order>,
    pub skip: Option<Expression>,
    pub limit: Option<Expression>,
    pub where_clause: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Return {
    pub distinct: bool,
    pub items: Vec<ProjectionItem>,
    pub order: Option<Order>,
    pub skip: Option<Expression>,
    pub limit: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionItem {
    pub expression: Expression,
    pub alias: Option<Variable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub items: Vec<SortItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SortItem {
    pub expression: Expression,
    pub direction: Option<SortDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unwind {
    pub expression: Expression,
    pub variable: Variable,
    pub span: Span,
}
