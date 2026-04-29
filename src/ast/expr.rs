use crate::ast::names::{PropertyKeyName, SymbolicName, Variable};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(NumberLiteral),
    String(StringLiteral),
    Boolean(bool),
    Null,
    List(ListLiteral),
    Map(MapLiteral),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberLiteral {
    Integer(i64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListLiteral {
    pub elements: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapLiteral {
    pub entries: Vec<(PropertyKeyName, Expression)>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: SymbolicName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Variable(Variable),
    Parameter(Parameter),
    PropertyLookup {
        base: Box<Expression>,
        property: PropertyKeyName,
        span: Span,
    },
    NodeLabels {
        base: Box<Expression>,
        labels: Vec<SymbolicName>,
        span: Span,
    },
    BinaryOp {
        op: BinaryOperator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
    Comparison {
        lhs: Box<Expression>,
        operators: Vec<(ComparisonOperator, Box<Expression>)>,
        span: Span,
    },
    ListIndex {
        list: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    ListSlice {
        list: Box<Expression>,
        start: Option<Box<Expression>>,
        end: Option<Box<Expression>>,
        span: Span,
    },
    In {
        lhs: Box<Expression>,
        rhs: Box<Expression>,
        span: Span,
    },
    IsNull {
        operand: Box<Expression>,
        negated: bool,
        span: Span,
    },
    FunctionCall(FunctionInvocation),
    CountStar {
        span: Span,
    },
    Case(CaseExpression),
    ListComprehension(Box<ListComprehension>),
    PatternComprehension(Box<PatternComprehension>),
    All(Box<FilterExpression>),
    Any(Box<FilterExpression>),
    None(Box<FilterExpression>),
    Single(Box<FilterExpression>),
    Parenthesized(Box<Expression>),
    Pattern(super::pattern::RelationshipsPattern),
    Exists(Box<ExistsExpression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Plus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    StartsWith,
    EndsWith,
    Contains,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInvocation {
    pub name: Vec<SymbolicName>,
    pub distinct: bool,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseExpression {
    pub scrutinee: Option<Box<Expression>>,
    pub alternatives: Vec<CaseAlternative>,
    pub default: Option<Box<Expression>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseAlternative {
    pub when: Expression,
    pub then: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListComprehension {
    pub variable: Variable,
    pub filter: Option<Box<Expression>>,
    pub map: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternComprehension {
    pub variable: Option<Variable>,
    pub pattern: super::pattern::RelationshipsPattern,
    pub where_clause: Option<Expression>,
    pub map: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterExpression {
    pub variable: Variable,
    pub collection: Box<Expression>,
    pub predicate: Option<Box<Expression>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExistsExpression {
    pub inner: Box<ExistsInner>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExistsInner {
    Pattern(super::pattern::Pattern, Option<Box<Expression>>),
    RegularQuery(super::query::RegularQuery),
}
