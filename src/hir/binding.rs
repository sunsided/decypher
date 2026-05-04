use super::arena::{BindingId, ScopeId};
use crate::error::Span;

/// A lexical scope in the HIR.
#[derive(Debug, Clone)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub bindings: Vec<BindingId>,
}

/// A named binding (variable) in the HIR.
#[derive(Debug, Clone)]
pub struct Binding {
    pub id: BindingId,
    pub name: String,
    pub kind: BindingKind,
    pub introduced_at: Span,
}

/// How a binding was introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// Bound by a graph pattern (node, relationship, or path).
    PatternBound,
    /// Bound by UNWIND.
    UnwindBound,
    /// Bound by WITH … AS alias.
    WithAlias,
    /// Bound by RETURN … AS alias.
    ReturnAlias,
    /// Bound by CALL … YIELD … AS alias.
    YieldAlias,
    /// Bound by FOREACH loop variable.
    ForeachVar,
    /// Bound inside a comprehension.
    ComprehensionVar,
    /// A scalar value (e.g. aggregation result, parameter).
    Value,
}
