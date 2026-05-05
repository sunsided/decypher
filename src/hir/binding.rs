//! Scope and variable-binding types for the HIR.
//!
//! The HIR models lexical scoping explicitly. Each [`Scope`] contains a list
//! of [`Binding`] IDs. Bindings represent named variables with their
//! introduction site and the semantic role in which they were bound.

use super::arena::{BindingId, ScopeId};
use crate::error::Span;

/// A lexical scope in the HIR.
///
/// Scopes form a tree: each scope optionally refers to a parent scope.
/// Variable resolution walks up the tree from the current scope.
#[derive(Debug, Clone)]
pub struct Scope {
    /// The enclosing scope, or `None` for the root scope.
    pub parent: Option<ScopeId>,
    /// IDs of the bindings directly introduced in this scope.
    pub bindings: Vec<BindingId>,
}

/// A named variable binding in the HIR.
///
/// Every variable occurrence in the query that has been successfully resolved
/// is backed by a `Binding` stored in the [`crate::hir::arena::HirArenas`].
#[derive(Debug, Clone)]
pub struct Binding {
    /// This binding's own arena ID (redundantly stored for convenience).
    pub id: BindingId,
    /// The variable name as it appears in the query source.
    pub name: String,
    /// How this binding was introduced.
    pub kind: BindingKind,
    /// The source location at which the variable was first introduced.
    pub introduced_at: Span,
}

/// The semantic role in which a variable was introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// Bound by a graph pattern (node, relationship, or path variable).
    PatternBound,
    /// Bound by `UNWIND … AS variable`.
    UnwindBound,
    /// Bound by `WITH … AS alias`.
    WithAlias,
    /// Bound by `RETURN … AS alias`.
    ReturnAlias,
    /// Bound by `CALL … YIELD … AS alias`.
    YieldAlias,
    /// Bound by a `FOREACH (variable IN list | …)` loop.
    ForeachVar,
    /// Bound inside a list/pattern comprehension or `ALL`/`ANY`/etc.
    ComprehensionVar,
    /// A scalar result (aggregation result, parameter, etc.).
    Value,
}
