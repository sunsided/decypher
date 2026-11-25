//! Normalised graph pattern types for the HIR.
//!
//! The HIR flattens the recursive AST graph pattern into parallel lists of
//! [`NodePattern`]s and [`RelationshipPattern`]s, linked by integer indices.
//! This representation is easier for an execution engine to process than the
//! recursive AST form.

use std::num::NonZeroU32;

use super::arena::{BindingId, ExprId, LabelId, RelTypeId};

/// A normalised, flattened graph pattern.
///
/// Nodes and relationships are stored in separate `Vec`s and cross-referenced
/// by index. Each comma-separated part of the original pattern is one entry of
/// `paths`, with its path variable and path selector.
#[derive(Debug, Clone, Default)]
pub struct GraphPattern {
    /// Flat list of node patterns.
    pub nodes: Vec<NodePattern>,
    /// Flat list of relationship patterns (each references two node indices).
    pub relationships: Vec<RelationshipPattern>,
    /// Quantified path patterns and quantified relationships. Each joins two
    /// nodes of `nodes` with a repeated sub-pattern.
    pub quantified: Vec<QuantifiedPattern>,
    /// The pattern parts, in source order. Each node and relationship belongs
    /// to exactly one of them, and so does each quantified group.
    ///
    /// The patterns of pattern expressions and pattern comprehensions have
    /// no parts and leave this empty.
    pub paths: Vec<PathPattern>,
}

impl GraphPattern {
    /// Create an empty pattern (no nodes, relationships, or path bindings).
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            relationships: Vec::new(),
            quantified: Vec::new(),
            paths: Vec::new(),
        }
    }
}

/// A label or relationship-type expression that is more than a plain list of
/// static names: alternatives, conjunctions, negation, the `%` wildcard or
/// dynamic labels.
#[derive(Debug, Clone, PartialEq)]
pub enum LabelExpr {
    /// A static name, interned in `labels` for node labels or in
    /// `relationship_types` for relationship types.
    Name(LabelId),
    /// `$(expr)`: a label or type computed at runtime.
    Dynamic(ExprId),
    /// `%`: any label or type.
    Wildcard,
    /// `lhs | rhs`
    Or(Box<LabelExpr>, Box<LabelExpr>),
    /// `lhs & rhs`
    And(Box<LabelExpr>, Box<LabelExpr>),
    /// `!inner`
    Not(Box<LabelExpr>),
}

/// A single node in a normalised graph pattern.
#[derive(Debug, Clone)]
pub struct NodePattern {
    /// Optional variable binding for this node.
    pub binding: Option<BindingId>,
    /// Required label IDs, when the node's labels are a plain conjunction of
    /// static labels (`(n:A:B)`); otherwise empty, see `label_expression`.
    pub labels: Vec<LabelId>,
    /// The node's label expression when it is more than a conjunction of
    /// static labels (`(n:A|B)`, `(n:!A)`, `(n:%)`, `(n:$(x))`).
    pub label_expression: Option<LabelExpr>,
    /// Optional property constraint expression.
    pub properties: Option<ExprId>,
    /// Optional inline predicate (`(n WHERE n.age > 25)`).
    ///
    /// A match of this node is valid only when the predicate is `true`. The
    /// predicate can use every variable of the enclosing pattern.
    pub predicate: Option<ExprId>,
}

/// A single relationship in a normalised graph pattern.
#[derive(Debug, Clone)]
pub struct RelationshipPattern {
    /// Optional variable binding for this relationship.
    pub binding: Option<BindingId>,
    /// Traversal direction.
    pub direction: RelationshipDirection,
    /// Index into [`GraphPattern::nodes`] for the left (start) node.
    pub left: NodeIndex,
    /// Index into [`GraphPattern::nodes`] for the right (end) node.
    pub right: NodeIndex,
    /// Allowed relationship type IDs when the type expression is a plain
    /// alternation of static types (`[:A|B]`); empty means any type, unless
    /// `type_expression` is set.
    pub types: Vec<RelTypeId>,
    /// The relationship's type expression when it is more than an alternation
    /// of static types (`[:!A]`, `[:A&B]`, `[:%]`, `[:$(x)]`).
    pub type_expression: Option<LabelExpr>,
    /// Variable-length range, or [`RelationshipLength::Single`] for fixed.
    pub length: RelationshipLength,
    /// Optional property constraint expression.
    pub properties: Option<ExprId>,
    /// Optional inline predicate (`[r WHERE r.since > 2020]`).
    ///
    /// A match of this relationship is valid only when the predicate is
    /// `true`. The predicate can use every variable of the enclosing pattern.
    pub predicate: Option<ExprId>,
}

/// One comma-separated part of a pattern: a path, with its optional path
/// variable and its path selector.
#[derive(Debug, Clone)]
pub struct PathPattern {
    /// The path variable (`p` in `p = (a)-->(b)`), or `None` when the part
    /// has none.
    pub binding: Option<BindingId>,
    /// Which of the matching paths to keep.
    pub selector: PathSelector,
    /// Ordered node indices that form the path.
    pub nodes: Vec<NodeIndex>,
    /// Ordered relationship indices that form the path.
    pub relationships: Vec<RelIndex>,
    /// Indices of the quantified groups of the path, in path order.
    ///
    /// The path goes through `nodes` in order. Two neighbouring nodes are
    /// joined by a relationship of `relationships` or by one of these groups.
    pub quantified: Vec<QuantifiedIndex>,
}

/// A quantified path pattern (`((x)-[:R]->(y)){1,3}`) or a quantified
/// relationship (`-[:R]->{1,3}`, lowered as `(()-[:R]->()){1,3}`).
///
/// It matches `min` to `max` repetitions of `inner`, chained end to start:
/// the last node of one repetition is the first node of the next. The first
/// node of the first repetition is the outer node `left`, and the last node
/// of the last repetition is the outer node `right`. With zero repetitions,
/// `left` and `right` are the same node.
#[derive(Debug, Clone)]
pub struct QuantifiedPattern {
    /// One repetition, with its own node and relationship indices. Its
    /// first node is `inner.nodes[0]` and its last node the last entry of
    /// its only path.
    pub inner: GraphPattern,
    /// The smallest number of repetitions.
    pub min: u32,
    /// The largest number of repetitions, or `None` for no limit.
    pub max: Option<u32>,
    /// The outer node where the repetitions start.
    pub left: NodeIndex,
    /// The outer node where the repetitions end.
    pub right: NodeIndex,
    /// The variables of `inner`, each with the list variable that holds
    /// its values outside the group.
    pub group_variables: Vec<GroupVariable>,
}

/// A variable of a quantified group: one value inside each repetition, and
/// a list of all of these values outside the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupVariable {
    /// The binding inside one repetition, used by `inner`.
    pub singleton: BindingId,
    /// The binding outside the group: the list of the singleton's values,
    /// one for each repetition, in path order.
    pub list: BindingId,
}

/// Which of the paths that match a pattern part to keep.
///
/// The spellings of the same selection lower to one variant: `shortestPath`
/// and `ANY SHORTEST` are `Shortest(1)`, `allShortestPaths` and
/// `ALL SHORTEST` are `ShortestGroups(1)`, and `ANY` is `Any(1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSelector {
    /// All matching paths: a plain pattern, or `ALL`.
    All,
    /// Any `k` matching paths (`ANY k`).
    Any(NonZeroU32),
    /// The `k` shortest paths (`SHORTEST k`). Paths of equal length are
    /// chosen in no particular order.
    Shortest(NonZeroU32),
    /// All paths of the `k` shortest lengths (`SHORTEST k GROUPS`).
    ShortestGroups(NonZeroU32),
}

/// Traversal direction of a relationship in the normalised pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    /// `(a)-[r]->(b)`: left to right.
    LeftToRight,
    /// `(a)<-[r]-(b)`: right to left.
    RightToLeft,
    /// `-[r]-`: no direction constraint.
    Undirected,
    /// `<-[r]->`: both directions.
    Both,
}

/// The hop-count multiplicity of a relationship pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipLength {
    /// Exactly one hop (the default).
    Single,
    /// Variable number of hops (`*`, `*2..5`, etc.).
    Variable {
        /// Minimum hops (inclusive), or `None` for 0.
        min: Option<u32>,
        /// Maximum hops (inclusive), or `None` for unbounded.
        max: Option<u32>,
    },
}

/// An index into [`GraphPattern::nodes`].
pub type NodeIndex = usize;
/// An index into [`GraphPattern::relationships`].
pub type RelIndex = usize;
/// An index into [`GraphPattern::quantified`].
pub type QuantifiedIndex = usize;
