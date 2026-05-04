use crate::ast::expr::Parameter;
use crate::ast::names::{RelTypeName, SymbolicName, Variable};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub parts: Vec<PatternPart>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternPart {
    pub variable: Option<Variable>,
    pub anonymous: AnonymousPatternPart,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousPatternPart {
    pub element: PatternElement,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternElement {
    Path {
        start: NodePattern,
        chains: Vec<PatternElementChain>,
    },
    Parenthesized(Box<PatternElement>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodePattern {
    pub variable: Option<Variable>,
    pub labels: Vec<SymbolicName>,
    pub properties: Option<Properties>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternElementChain {
    pub relationship: RelationshipPattern,
    pub node: NodePattern,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipPattern {
    pub direction: RelationshipDirection,
    pub detail: Option<RelationshipDetail>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    Left,
    Right,
    Both,
    Undirected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipDetail {
    pub variable: Option<Variable>,
    pub types: Vec<RelTypeName>,
    pub range: Option<RangeLiteral>,
    pub properties: Option<Properties>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeLiteral {
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipsPattern {
    pub start: NodePattern,
    pub chains: Vec<PatternElementChain>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Properties {
    Map(crate::ast::expr::MapLiteral),
    Parameter(Parameter),
}
