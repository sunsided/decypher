//! Graph pattern AST nodes for openCypher queries.
//!
//! A graph pattern describes the structural shapes that the query engine
//! matches against the graph. The top-level type is [`Pattern`], which
//! holds one or more named or anonymous [`PatternPart`]s. Each part is a
//! chain of alternating [`NodePattern`]s and [`RelationshipPattern`]s.

use crate::ast::expr::{Expression, Parameter};
use crate::ast::names::{SymbolicName, Variable};
use crate::error::Span;

/// A full graph pattern: one or more comma-separated pattern parts.
///
/// Appears after `MATCH`, `CREATE`, `MERGE`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// The individual pattern parts, in order.
    pub parts: Vec<PatternPart>,
    /// Byte-offset span of the entire pattern.
    pub span: Span,
}

/// A single pattern part, optionally bound to a path variable.
///
/// For example, `p = (a)-[:KNOWS]->(b)` binds the matched path to `p`.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternPart {
    /// Optional path variable (`p = ...`).
    pub variable: Option<Variable>,
    /// The anonymous path pattern.
    pub anonymous: AnonymousPatternPart,
    /// Byte-offset span of this part.
    pub span: Span,
}

/// An anonymous pattern part (the path without an optional path variable).
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousPatternPart {
    /// The root pattern element.
    pub element: PatternElement,
}

/// A pattern element: a node-anchored path, a parenthesized group, or a
/// quantified subpattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternElement {
    /// A concrete path: a start node followed by relationship/node chains.
    Path {
        /// The starting node pattern.
        start: NodePattern,
        /// Zero or more relationship/node extensions.
        chains: Vec<PatternElementChain>,
    },
    /// A parenthesized subpattern `(element)`.
    Parenthesized(Box<PatternElement>),
    /// A quantified pattern `element{n,m}`.
    Quantified {
        /// The repeated sub-element.
        element: Box<PatternElement>,
        /// The quantifier bounds.
        quantifier: Quantifier,
        /// Byte-offset span of the quantified pattern.
        span: Span,
    },
}

/// A node pattern: `(variable:Label {properties})`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodePattern {
    /// Optional binding variable for this node.
    pub variable: Option<Variable>,
    /// Zero or more label expressions (`Person`, `Person&!Deleted`, `$(dyn)`, …).
    pub labels: Vec<LabelExpression>,
    /// Optional property map or parameter constraint.
    pub properties: Option<Properties>,
    /// Byte-offset span of the node pattern.
    pub span: Span,
}

/// A relationship/node extension in a path: one relationship followed by a node.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternElementChain {
    /// The relationship traversal.
    pub relationship: RelationshipPattern,
    /// The destination node pattern.
    pub node: NodePattern,
}

/// A relationship pattern: `[variable:TYPE*range {properties}]` with direction.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipPattern {
    /// The traversal direction.
    pub direction: RelationshipDirection,
    /// Inner detail (`variable`, `types`, `range`, `properties`), or `None`
    /// for an anonymous bare relationship `--`.
    pub detail: Option<RelationshipDetail>,
    /// Optional quantifier (GQL extension, e.g. `-[]->{1,3}`).
    pub quantifier: Option<Quantifier>,
    /// Byte-offset span of the relationship pattern.
    pub span: Span,
}

/// Traversal direction for a [`RelationshipPattern`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    /// `<--` or `<-[…]-`
    Left,
    /// `-->` or `-[…]->`
    Right,
    /// `<-->` or `<-[…]->`
    Both,
    /// `--` or `-[…]-` (no arrowhead)
    Undirected,
}

/// The inner detail of a relationship pattern: variable, types, range, props.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipDetail {
    /// Optional binding variable for this relationship.
    pub variable: Option<Variable>,
    /// Optional type expression (e.g. `KNOWS|LIKES`, `!FOLLOWS`).
    pub types: Option<LabelExpression>,
    /// Optional variable-length range `*min..max`.
    pub range: Option<RangeLiteral>,
    /// Optional property map or parameter constraint.
    pub properties: Option<Properties>,
    /// Byte-offset span of the relationship detail.
    pub span: Span,
}

/// A variable-length range literal for relationships: `*min..max`.
///
/// Either bound may be `None` to represent an unbounded range (`*`, `*..5`,
/// `*2..`).
#[derive(Debug, Clone, PartialEq)]
pub struct RangeLiteral {
    /// Minimum number of hops (inclusive), or `None` for 0/unspecified.
    pub start: Option<i64>,
    /// Maximum number of hops (inclusive), or `None` for unbounded.
    pub end: Option<i64>,
    /// Byte-offset span of the range literal.
    pub span: Span,
}

/// A quantifier bound for quantified path patterns: `{min,max}`.
///
/// Either bound may be `None` for an unspecified / unbounded value.
#[derive(Debug, Clone, PartialEq)]
pub struct Quantifier {
    /// Minimum number of repetitions, or `None`.
    pub start: Option<i64>,
    /// Maximum number of repetitions, or `None` for unbounded.
    pub end: Option<i64>,
    /// Byte-offset span of the quantifier.
    pub span: Span,
}

/// A relationships pattern used as an expression (e.g. inside `EXISTS`).
///
/// Unlike the top-level [`Pattern`], this form must start from a node and
/// include at least one relationship chain.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipsPattern {
    /// The starting node pattern.
    pub start: NodePattern,
    /// The relationship/node chains.
    pub chains: Vec<PatternElementChain>,
    /// Byte-offset span of the pattern expression.
    pub span: Span,
}

/// Property constraints on a node or relationship pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Properties {
    /// A literal map `{key: value, …}`.
    Map(crate::ast::expr::MapLiteral),
    /// A parameter `$param` that must be a map value at runtime.
    Parameter(Parameter),
}

/// A label or relationship-type expression, supporting boolean algebra.
///
/// openCypher supports compound label expressions:
/// - `Person` (static)
/// - `$(labelVar)` (dynamic)
/// - `Person|Company` (or)
/// - `Person&!Deleted` (and / not)
/// - `(Person|Company)` (parenthesised group)
#[derive(Debug, Clone, PartialEq)]
pub enum LabelExpression {
    /// A plain static label/type name.
    Static(SymbolicName),
    /// A dynamic label expression `$(expr)`.
    Dynamic {
        /// The expression that evaluates to a label string at runtime.
        expression: Box<Expression>,
        /// Byte-offset span of the `$(…)` construct.
        span: Span,
    },
    /// `lhs | rhs` — either label.
    Or {
        /// Left-hand label expression.
        lhs: Box<LabelExpression>,
        /// Right-hand label expression.
        rhs: Box<LabelExpression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// `lhs & rhs` — both labels.
    And {
        /// Left-hand label expression.
        lhs: Box<LabelExpression>,
        /// Right-hand label expression.
        rhs: Box<LabelExpression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// `!inner` — the absence of a label.
    Not {
        /// The negated label expression.
        inner: Box<LabelExpression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
    /// `(inner)` — a parenthesised group for disambiguation.
    Group {
        /// The inner label expression.
        inner: Box<LabelExpression>,
        /// Byte-offset span of the expression.
        span: Span,
    },
}
