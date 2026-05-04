use super::arena::{BindingId, ExprId, LabelId, RelTypeId};

/// A normalized graph pattern: flat lists of nodes and relationships.
#[derive(Debug, Clone)]
pub struct GraphPattern {
    pub nodes: Vec<NodePattern>,
    pub relationships: Vec<RelationshipPattern>,
    pub path_bindings: Vec<PathBinding>,
}

impl GraphPattern {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            relationships: Vec::new(),
            path_bindings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodePattern {
    pub binding: Option<BindingId>,
    pub labels: Vec<LabelId>,
    pub properties: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct RelationshipPattern {
    pub binding: Option<BindingId>,
    pub direction: RelationshipDirection,
    pub left: NodeIndex,
    pub right: NodeIndex,
    pub types: Vec<RelTypeId>,
    pub length: RelationshipLength,
    pub properties: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct PathBinding {
    pub binding: BindingId,
    pub nodes: Vec<NodeIndex>,
    pub relationships: Vec<RelIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    LeftToRight,
    RightToLeft,
    Undirected,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipLength {
    Single,
    Variable { min: Option<u32>, max: Option<u32> },
}

pub type NodeIndex = usize;
pub type RelIndex = usize;
