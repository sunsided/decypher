//! HIR expression node types.
//!
//! The central type is [`HirExpr`], which wraps an [`ExprKind`] and a source
//! [`Span`]. All sub-expressions are referenced by [`ExprId`] arena handles
//! rather than by pointer, keeping the representation flat and cheap to clone.

use super::arena::{BindingId, ExprId, FunctionId, LabelId, ParameterId, PropertyKeyId};
use super::pattern::GraphPattern;
use crate::error::Span;

/// A single HIR expression node, stored in the expression arena.
#[derive(Debug, Clone)]
pub struct HirExpr {
    /// The expression kind with its payload.
    pub kind: ExprKind,
    /// Source location of this expression.
    pub span: Span,
}

/// The payload of a [`HirExpr`] node.
///
/// Variants use arena IDs ([`ExprId`], [`BindingId`], etc.) instead of
/// owned values to keep the HIR flat.
#[derive(Debug, Clone)]
pub enum ExprKind {
    /// A compile-time literal value.
    Literal(Literal),
    /// A reference to a bound variable.
    Binding(BindingId),
    /// Property access: `base.key`.
    Property {
        /// The base expression.
        base: ExprId,
        /// The interned property key.
        key: PropertyKeyId,
    },
    /// A query parameter reference (`$name`).
    Parameter(ParameterId),
    /// A list literal `[e1, e2, …]`.
    List(Vec<ExprId>),
    /// A map literal `{k1: v1, k2: v2, …}`.
    Map(Vec<(PropertyKeyId, ExprId)>),

    /// A unary operation.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand expression.
        expr: ExprId,
    },
    /// A binary infix operation.
    Binary {
        /// The operator.
        op: BinaryOp,
        /// The left operand.
        left: ExprId,
        /// The right operand.
        right: ExprId,
    },

    /// A function or aggregate call.
    FunctionCall {
        /// The interned function name.
        function: FunctionId,
        /// The positional argument expressions.
        args: Vec<ExprId>,
        /// `true` when called as `f(DISTINCT …)`.
        distinct: bool,
    },

    /// A `CASE … END` expression.
    Case(CaseExpr),
    /// A pattern comprehension `[(var)-[r]->… | map]`.
    PatternComprehension(PatternComprehension),
    /// A list comprehension `[var IN list [WHERE pred] | map]`.
    ListComprehension(ListComprehension),
    /// An `EXISTS { … }` subquery expression.
    ExistsSubquery(ExistsSubquery),
    /// A `COUNT { … }` subquery expression.
    CountSubquery(CountSubquery),
    /// A `COLLECT { … }` subquery expression.
    CollectSubquery(CollectSubquery),

    /// A pattern expression used inside `EXISTS` or a pattern comprehension.
    PatternExpr(GraphPattern),

    /// `count(*)` — the special aggregate form.
    CountStar,

    /// `ALL` / `ANY` / `NONE` / `SINGLE` predicate over a collection.
    CollectionFilter {
        /// Which quantifier is applied.
        quantifier: CollectionQuantifier,
        /// The element variable.
        variable: BindingId,
        /// The collection expression.
        collection: ExprId,
        /// Optional `WHERE` predicate.
        predicate: Option<ExprId>,
    },

    /// Node labels check/assignment: `base:Label`.
    NodeLabels {
        /// The base expression (typically a node variable).
        base: ExprId,
        /// The labels to check.
        labels: Vec<LabelId>,
    },

    /// List index access: `list[i]`.
    ListIndex {
        /// The list expression.
        list: ExprId,
        /// The index expression.
        index: ExprId,
    },

    /// List slice: `list[start..end]`.
    ListSlice {
        /// The list expression.
        list: ExprId,
        /// Inclusive start index.
        start: Option<ExprId>,
        /// Exclusive end index.
        end: Option<ExprId>,
    },

    /// `lhs IN rhs` membership test.
    In {
        /// The element to test.
        lhs: ExprId,
        /// The list to test against.
        rhs: ExprId,
    },

    /// `IS NULL` / `IS NOT NULL` predicate.
    IsNull {
        /// The expression whose nullity is tested.
        operand: ExprId,
        /// `true` for `IS NOT NULL`.
        negated: bool,
    },

    /// A chained comparison: `left op1 rhs1 op2 rhs2 …`.
    Comparison {
        /// The leftmost operand.
        left: ExprId,
        /// The subsequent `(operator, right_operand)` pairs.
        operators: Vec<(ComparisonOperator, ExprId)>,
    },

    /// A map projection: `base { item, … }`.
    MapProjection {
        /// The base variable.
        base: BindingId,
        /// The projected items.
        items: Vec<MapProjectionItem>,
    },
}

/// A literal value in the HIR.
#[derive(Debug, Clone)]
pub enum Literal {
    /// `null`
    Null,
    /// `true` / `false`
    Boolean(bool),
    /// A 64-bit signed integer.
    Integer(i64),
    /// A 64-bit IEEE 754 float.
    Float(f64),
    /// A decoded string value.
    String(String),
}

/// Unary operators in the HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Arithmetic negation (`-expr`).
    Negate,
    /// Unary plus (`+expr`, no-op).
    Plus,
    /// Logical negation (`NOT expr`).
    Not,
}

/// Binary infix operators in the HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `%`
    Modulo,
    /// `^`
    Power,
    /// `AND`
    And,
    /// `OR`
    Or,
    /// `XOR`
    Xor,
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `=~` (regex match)
    RegexMatch,
    /// `STARTS WITH`
    StartsWith,
    /// `ENDS WITH`
    EndsWith,
    /// `CONTAINS`
    Contains,
    /// `IN`
    In,
    /// `IS`
    Is,
    /// `IS NOT`
    IsNot,
}

/// Comparison operators used in chained comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `=~`
    RegexMatch,
    /// `STARTS WITH`
    StartsWith,
    /// `ENDS WITH`
    EndsWith,
    /// `CONTAINS`
    Contains,
}

/// The quantifier applied by `ALL`, `ANY`, `NONE`, or `SINGLE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionQuantifier {
    /// `ALL(x IN list WHERE pred)` — all elements satisfy the predicate.
    All,
    /// `ANY(x IN list WHERE pred)` — at least one element satisfies.
    Any,
    /// `NONE(x IN list WHERE pred)` — no element satisfies.
    None,
    /// `SINGLE(x IN list WHERE pred)` — exactly one element satisfies.
    Single,
}

/// An `EXISTS { subquery }` expression in the HIR.
#[derive(Debug, Clone)]
pub struct ExistsSubquery {
    /// The inner subquery.
    pub query: Box<super::HirQuery>,
    /// Bindings from the outer scope imported into the subquery.
    pub imported_bindings: Vec<BindingId>,
}

/// A `COUNT { subquery }` expression in the HIR.
#[derive(Debug, Clone)]
pub struct CountSubquery {
    /// The inner subquery.
    pub query: Box<super::HirQuery>,
    /// Bindings from the outer scope imported into the subquery.
    pub imported_bindings: Vec<BindingId>,
}

/// A `COLLECT { subquery }` expression in the HIR.
#[derive(Debug, Clone)]
pub struct CollectSubquery {
    /// The inner subquery.
    pub query: Box<super::HirQuery>,
    /// Bindings from the outer scope imported into the subquery.
    pub imported_bindings: Vec<BindingId>,
}

/// A list comprehension expression.
#[derive(Debug, Clone)]
pub struct ListComprehension {
    /// The element variable.
    pub variable: BindingId,
    /// The source collection expression.
    pub collection: ExprId,
    /// Optional `WHERE` filter.
    pub filter: Option<ExprId>,
    /// The map expression, or `None` for identity.
    pub map: Option<ExprId>,
}

/// A pattern comprehension expression.
#[derive(Debug, Clone)]
pub struct PatternComprehension {
    /// Optional path variable.
    pub variable: Option<BindingId>,
    /// The path pattern to match.
    pub pattern: GraphPattern,
    /// Optional `WHERE` filter.
    pub filter: Option<ExprId>,
    /// The map expression applied to each match.
    pub map: ExprId,
}

/// A `CASE` expression in the HIR.
#[derive(Debug, Clone)]
pub struct CaseExpr {
    /// The scrutinee for generic `CASE expr WHEN …`, or `None` for searched form.
    pub scrutinee: Option<ExprId>,
    /// The `WHEN … THEN …` alternatives.
    pub alternatives: Vec<CaseAlternative>,
    /// The `ELSE` default, or `None`.
    pub default: Option<ExprId>,
}

/// One `WHEN condition THEN result` branch in a [`CaseExpr`].
#[derive(Debug, Clone)]
pub struct CaseAlternative {
    /// The `WHEN` condition expression.
    pub when: ExprId,
    /// The `THEN` result expression.
    pub then: ExprId,
}

/// A single item in a map projection expression.
#[derive(Debug, Clone)]
pub enum MapProjectionItem {
    /// `.*` — all properties of the base entity.
    AllProperties,
    /// `.key` — one property by key.
    PropertyLookup {
        /// The interned property key.
        key: PropertyKeyId,
    },
    /// `key: value` — a literal key–value entry.
    Literal {
        /// The interned key name.
        key: PropertyKeyId,
        /// The value expression.
        value: ExprId,
    },
}
