//! Normalised graph pattern types for the HIR.
//!
//! The HIR flattens the recursive AST graph pattern into parallel lists of
//! [`NodePattern`]s and [`RelationshipPattern`]s, linked by integer indices.
//! This representation is easier for an execution engine to process than the
//! recursive AST form.

use super::arena::{BindingId, ExprId, LabelId, RelTypeId};

/// A normalised, flattened graph pattern.
///
/// Nodes and relationships are stored in separate `Vec`s and cross-referenced
/// by index. All path bindings from the original pattern are also captured.
#[derive(Debug, Clone)]
pub struct GraphPattern {
    /// Flat list of node patterns.
    pub nodes: Vec<NodePattern>,
    /// Flat list of relationship patterns (each references two node indices).
    pub relationships: Vec<RelationshipPattern>,
    /// Path-variable bindings that span subsets of the node/relationship lists.
    pub path_bindings: Vec<PathBinding>,
}

impl GraphPattern {
    /// Create an empty pattern (no nodes, relationships, or path bindings).
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            relationships: Vec::new(),
            path_bindings: Vec::new(),
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
}

/// A path variable that captures a sub-path in the pattern.
#[derive(Debug, Clone)]
pub struct PathBinding {
    /// The binding that receives the path value.
    pub binding: BindingId,
    /// Ordered node indices that form the path.
    pub nodes: Vec<NodeIndex>,
    /// Ordered relationship indices that form the path.
    pub relationships: Vec<RelIndex>,
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
