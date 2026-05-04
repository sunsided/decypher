use crate::ast::expr::{Expression, FunctionInvocation};
use crate::ast::names::Variable;
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct StandaloneCall {
    pub call: ProcedureInvocation,
    pub yield_items: Option<YieldSpec>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InQueryCall {
    pub call: ProcedureInvocation,
    pub yield_items: Option<YieldItems>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureInvocation {
    pub name: FunctionInvocation,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum YieldSpec {
    Star { span: Span },
    Items(YieldItems),
}

#[derive(Debug, Clone, PartialEq)]
pub struct YieldItems {
    pub items: Vec<YieldItem>,
    pub where_clause: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct YieldItem {
    pub procedure_field: crate::ast::names::SymbolicName,
    pub alias: Option<Variable>,
}
