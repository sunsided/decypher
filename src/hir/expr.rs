use super::arena::{BindingId, ExprId, FunctionId, LabelId, ParameterId, PropertyKeyId};
use super::pattern::GraphPattern;
use crate::error::Span;

/// A HIR expression node.
#[derive(Debug, Clone)]
pub struct HirExpr {
    pub kind: ExprKind,
    pub span: Span,
}

/// Kinds of HIR expressions.
#[derive(Debug, Clone)]
pub enum ExprKind {
    Literal(Literal),
    Binding(BindingId),
    Property {
        base: ExprId,
        key: PropertyKeyId,
    },
    Parameter(ParameterId),
    List(Vec<ExprId>),
    Map(Vec<(PropertyKeyId, ExprId)>),

    Unary {
        op: UnaryOp,
        expr: ExprId,
    },
    Binary {
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
    },

    FunctionCall {
        function: FunctionId,
        args: Vec<ExprId>,
        distinct: bool,
    },

    Case(CaseExpr),
    PatternComprehension(PatternComprehension),
    ListComprehension(ListComprehension),
    ExistsSubquery(ExistsSubquery),
    CountSubquery(CountSubquery),
    CollectSubquery(CollectSubquery),

    /// Pattern expression inside EXISTS or pattern comprehension.
    PatternExpr(GraphPattern),

    /// count(*) — special form.
    CountStar,

    /// ALL / ANY / NONE / SINGLE predicate over a collection.
    CollectionFilter {
        quantifier: CollectionQuantifier,
        variable: BindingId,
        collection: ExprId,
        predicate: Option<ExprId>,
    },

    /// Node labels expression (e.g. `n:Person`).
    NodeLabels {
        base: ExprId,
        labels: Vec<LabelId>,
    },

    /// List index access.
    ListIndex {
        list: ExprId,
        index: ExprId,
    },

    /// List slice.
    ListSlice {
        list: ExprId,
        start: Option<ExprId>,
        end: Option<ExprId>,
    },

    /// IN operator.
    In {
        lhs: ExprId,
        rhs: ExprId,
    },

    /// IS NULL / IS NOT NULL.
    IsNull {
        operand: ExprId,
        negated: bool,
    },

    /// Comparison chain (a = b > c).
    Comparison {
        left: ExprId,
        operators: Vec<(ComparisonOperator, ExprId)>,
    },

    /// Map projection.
    MapProjection {
        base: BindingId,
        items: Vec<MapProjectionItem>,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Plus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    RegexMatch,
    StartsWith,
    EndsWith,
    Contains,
    In,
    Is,
    IsNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    RegexMatch,
    StartsWith,
    EndsWith,
    Contains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionQuantifier {
    All,
    Any,
    None,
    Single,
}

#[derive(Debug, Clone)]
pub struct ExistsSubquery {
    pub query: Box<super::HirQuery>,
    pub imported_bindings: Vec<BindingId>,
}

#[derive(Debug, Clone)]
pub struct CountSubquery {
    pub query: Box<super::HirQuery>,
    pub imported_bindings: Vec<BindingId>,
}

#[derive(Debug, Clone)]
pub struct CollectSubquery {
    pub query: Box<super::HirQuery>,
    pub imported_bindings: Vec<BindingId>,
}

#[derive(Debug, Clone)]
pub struct ListComprehension {
    pub variable: BindingId,
    pub collection: ExprId,
    pub filter: Option<ExprId>,
    pub map: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct PatternComprehension {
    pub variable: Option<BindingId>,
    pub pattern: GraphPattern,
    pub filter: Option<ExprId>,
    pub map: ExprId,
}

#[derive(Debug, Clone)]
pub struct CaseExpr {
    pub scrutinee: Option<ExprId>,
    pub alternatives: Vec<CaseAlternative>,
    pub default: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct CaseAlternative {
    pub when: ExprId,
    pub then: ExprId,
}

#[derive(Debug, Clone)]
pub enum MapProjectionItem {
    AllProperties,
    PropertyLookup { key: PropertyKeyId },
    Literal { key: PropertyKeyId, value: ExprId },
}
