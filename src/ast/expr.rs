//! Expression AST nodes for Cypher queries.
//!
//! The central type is [`Expression`], which is a recursive enum covering
//! every expression form defined in the Cypher specification: literals,
//! variables, operators, function calls, list/map constructors, subqueries,
//! and more.

use crate::ast::names::{PropertyKeyName, SymbolicName, Variable};
use crate::ast::pattern::LabelExpression;
use crate::error::Span;

/// A literal value in a Cypher expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer or float numeric literal.
    Number(NumberLiteral),
    /// A string literal.
    String(StringLiteral),
    /// `true` or `false`.
    Boolean(bool),
    /// The `null` literal.
    Null,
    /// `[e1, e2, …]` list literal.
    List(ListLiteral),
    /// `{key: value, …}` map literal.
    Map(MapLiteral),
}

/// A numeric literal: either an integer or a floating-point value.
#[derive(Debug, Clone, PartialEq)]
pub enum NumberLiteral {
    /// A 64-bit signed integer.
    Integer(i64),
    /// A 64-bit IEEE 754 float.
    Float(f64),
}

/// A string literal with its source span and optional raw form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    /// The decoded string value (surrounding quotes and escape sequences
    /// processed).
    pub value: String,
    /// Byte-offset span of the literal token in the original source.
    pub span: Span,
    /// The raw token text as it appeared in the source, if retained.
    pub raw: Option<String>,
}

/// `[e1, e2, …]` — a list literal expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ListLiteral {
    /// The element expressions, in order.
    pub elements: Vec<Expression>,
    /// Byte-offset span of the literal.
    pub span: Span,
}

/// `{key: value, …}` — a map literal expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLiteral {
    /// The key–value pairs, in source order.
    pub entries: Vec<(PropertyKeyName, Expression)>,
    /// Byte-offset span of the literal.
    pub span: Span,
}

/// A query parameter reference, e.g. `$name` or `$0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// The parameter name (without the leading `$`).
    pub name: SymbolicName,
    /// Byte-offset span of the `$name` token.
    pub span: Span,
}

/// Any Cypher expression.
///
/// This enum covers every expression form in the language. Recursive variants
/// (`BinaryOp`, `UnaryOp`, etc.) use `Box<Expression>` to avoid infinite
/// size.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A literal value (`42`, `'hello'`, `true`, `null`, `[1,2]`, `{k:v}`).
    Literal(Literal),
    /// A variable reference (e.g. `n`, `r`).
    Variable(Variable),
    /// A query parameter reference (e.g. `$name`).
    Parameter(Parameter),
    /// Property lookup: `base.property`.
    PropertyLookup {
        /// The expression whose property is accessed.
        base: Box<Expression>,
        /// The property key name.
        property: PropertyKeyName,
        /// Byte-offset span of the whole lookup expression.
        span: Span,
    },
    /// Label check or assignment: `base:Label1&Label2`.
    NodeLabels {
        /// The expression being labelled (typically a variable).
        base: Box<Expression>,
        /// The label expressions.
        labels: Vec<LabelExpression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// A binary infix operation: `lhs op rhs`.
    BinaryOp {
        /// The operator.
        op: BinaryOperator,
        /// The left-hand side operand.
        lhs: Box<Expression>,
        /// The right-hand side operand.
        rhs: Box<Expression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// A unary prefix operation: `op operand`.
    UnaryOp {
        /// The operator.
        op: UnaryOperator,
        /// The operand.
        operand: Box<Expression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// A chained comparison: `lhs op1 rhs1 op2 rhs2 …`
    ///
    /// Cypher allows chaining comparisons which are equivalent to
    /// `lhs op1 rhs1 AND rhs1 op2 rhs2`.
    Comparison {
        /// The leftmost operand.
        lhs: Box<Expression>,
        /// The remaining `(operator, right_operand)` pairs.
        operators: Vec<(ComparisonOperator, Box<Expression>)>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// Index access: `list[index]`.
    ListIndex {
        /// The list expression.
        list: Box<Expression>,
        /// The index expression.
        index: Box<Expression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// Slice access: `list[start..end]`.
    ///
    /// Either bound may be omitted (`list[..end]`, `list[start..]`).
    ListSlice {
        /// The list expression.
        list: Box<Expression>,
        /// Inclusive start index, or `None` for `0`.
        start: Option<Box<Expression>>,
        /// Exclusive end index, or `None` for the list length.
        end: Option<Box<Expression>>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// The `IN` operator: `lhs IN rhs`.
    In {
        /// The left-hand side expression.
        lhs: Box<Expression>,
        /// The list expression to test membership against.
        rhs: Box<Expression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// `IS NULL` / `IS NOT NULL` predicate.
    IsNull {
        /// The expression whose nullity is tested.
        operand: Box<Expression>,
        /// `true` for `IS NOT NULL`; `false` for `IS NULL`.
        negated: bool,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// A function invocation: `f(args)` or `ns.f(args)`.
    FunctionCall(FunctionInvocation),
    /// `count(*)` — the special aggregate form.
    CountStar {
        /// Byte-offset span of the `count(*)` expression.
        span: Span,
    },
    /// A `CASE WHEN … THEN … [ELSE …] END` expression.
    Case(CaseExpression),
    /// `[variable IN collection [WHERE predicate] | map]`
    ListComprehension(Box<ListComprehension>),
    /// `[(variable)-[r]->… [WHERE predicate] | map]`
    PatternComprehension(Box<PatternComprehension>),
    /// `ALL(variable IN collection [WHERE predicate])`
    All(Box<FilterExpression>),
    /// `ANY(variable IN collection [WHERE predicate])`
    Any(Box<FilterExpression>),
    /// `NONE(variable IN collection [WHERE predicate])`
    None(Box<FilterExpression>),
    /// `SINGLE(variable IN collection [WHERE predicate])`
    Single(Box<FilterExpression>),
    /// A parenthesized expression `(expr)`.
    Parenthesized(Box<Expression>),
    /// A relationships pattern used as an expression (e.g. inside `EXISTS`).
    Pattern(super::pattern::RelationshipsPattern),
    /// `EXISTS { … }` subquery expression.
    Exists(Box<ExistsExpression>),
    /// `COUNT { … }` subquery expression.
    CountSubquery(Box<CountSubqueryExpression>),
    /// `COLLECT { … }` subquery expression.
    CollectSubquery(Box<CollectSubqueryExpression>),
    /// `variable { … }` map projection expression.
    MapProjection(Box<MapProjection>),
}

/// Binary infix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
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
}

/// Unary prefix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// Arithmetic negation (`-expr`).
    Negate,
    /// Unary plus (`+expr`, no-op).
    Plus,
    /// Logical negation (`NOT expr`).
    Not,
}

/// Comparison / string / list operators used in chained comparisons.
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
    /// `=~` (regular-expression match)
    RegexMatch,
    /// `STARTS WITH`
    StartsWith,
    /// `ENDS WITH`
    EndsWith,
    /// `CONTAINS`
    Contains,
}

/// A function or procedure invocation: `[namespace.]name(args)`.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInvocation {
    /// The qualified function name parts, e.g. `["apoc", "text", "join"]`.
    pub name: Vec<SymbolicName>,
    /// `true` when called as `f(DISTINCT …)`.
    pub distinct: bool,
    /// Positional argument expressions.
    pub arguments: Vec<Expression>,
    /// Byte-offset span of the invocation.
    pub span: Span,
}

/// A `CASE` expression.
///
/// In *generic* form (`CASE expr WHEN … THEN …`) the `scrutinee` holds the
/// tested expression; in *searched* form (`CASE WHEN predicate THEN …`) the
/// `scrutinee` is `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseExpression {
    /// The expression tested in a generic `CASE expr WHEN …` form, or `None`
    /// for a searched `CASE WHEN predicate …` form.
    pub scrutinee: Option<Box<Expression>>,
    /// The `WHEN … THEN …` branches, in order.
    pub alternatives: Vec<CaseAlternative>,
    /// The `ELSE …` fallback, or `None` if omitted.
    pub default: Option<Box<Expression>>,
    /// Byte-offset span of the `CASE … END` expression.
    pub span: Span,
}

/// One `WHEN condition THEN result` branch of a [`CaseExpression`].
#[derive(Debug, Clone, PartialEq)]
pub struct CaseAlternative {
    /// The `WHEN` condition (or equality test in generic `CASE`).
    pub when: Expression,
    /// The `THEN` result expression.
    pub then: Expression,
}

/// `[variable IN collection [WHERE predicate] | map]`
///
/// Creates a new list by mapping (and optionally filtering) an existing one.
#[derive(Debug, Clone, PartialEq)]
pub struct ListComprehension {
    /// The element variable bound to each item in the source list.
    pub variable: Variable,
    /// Optional `WHERE` filter on elements.
    pub filter: Option<Box<Expression>>,
    /// The `| map` expression, or `None` for identity (`[x IN list]`).
    pub map: Option<Expression>,
    /// Byte-offset span of the comprehension.
    pub span: Span,
}

/// `[(variable)-[r]->… [WHERE predicate] | map]`
///
/// Builds a list of values by matching a path pattern and mapping each match.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternComprehension {
    /// Optional path variable that captures the matched path.
    pub variable: Option<Variable>,
    /// The path pattern to match.
    pub pattern: super::pattern::RelationshipsPattern,
    /// Optional `WHERE` filter applied to each matched path.
    pub where_clause: Option<Expression>,
    /// The map expression applied to each matched path.
    pub map: Expression,
    /// Byte-offset span of the comprehension.
    pub span: Span,
}

/// A filter predicate used by `ALL`, `ANY`, `NONE`, and `SINGLE`.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterExpression {
    /// The element variable bound to each item in the collection.
    pub variable: Variable,
    /// The collection expression.
    pub collection: Box<Expression>,
    /// Optional `WHERE` predicate tested for each element.
    pub predicate: Option<Box<Expression>>,
    /// Byte-offset span of the filter expression.
    pub span: Span,
}

/// `EXISTS { pattern | subquery }` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ExistsExpression {
    /// The inner pattern or subquery.
    pub inner: Box<ExistsInner>,
    /// Byte-offset span of the `EXISTS { … }` expression.
    pub span: Span,
}

/// `COUNT { subquery }` expression — returns the number of matching rows.
#[derive(Debug, Clone, PartialEq)]
pub struct CountSubqueryExpression {
    /// The inner subquery.
    pub query: Box<super::query::RegularQuery>,
    /// Byte-offset span of the `COUNT { … }` expression.
    pub span: Span,
}

/// `COLLECT { subquery }` expression — collects matching rows into a list.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectSubqueryExpression {
    /// The inner subquery.
    pub query: Box<super::query::RegularQuery>,
    /// Byte-offset span of the `COLLECT { … }` expression.
    pub span: Span,
}

/// The inner content of an `EXISTS { … }` expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExistsInner {
    /// A graph pattern with an optional `WHERE` predicate.
    Pattern(super::pattern::Pattern, Option<Box<Expression>>),
    /// A full subquery.
    RegularQuery(Box<super::query::RegularQuery>),
}

/// `variable { item, … }` map projection expression.
///
/// Creates a map from selected properties or literal entries of an existing
/// entity.
#[derive(Debug, Clone, PartialEq)]
pub struct MapProjection {
    /// The base variable (an existing node or map).
    pub base: Variable,
    /// The items to project into the new map.
    pub items: Vec<MapProjectionItem>,
    /// Byte-offset span of the projection.
    pub span: Span,
}

/// A single item in a [`MapProjection`].
#[derive(Debug, Clone, PartialEq)]
pub enum MapProjectionItem {
    /// `.*` — include all properties of the base entity.
    AllProperties {
        /// Byte-offset span of the `.*` token.
        span: Span,
    },
    /// `.property` — project one property by name.
    PropertyLookup {
        /// The property key to include.
        property: PropertyKeyName,
    },
    /// `key: value` — a literal key–value entry.
    Literal {
        /// The literal key name.
        key: PropertyKeyName,
        /// The value expression.
        value: Expression,
    },
}
