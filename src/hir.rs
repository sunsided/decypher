#![cfg_attr(docsrs, doc(cfg(feature = "hir")))]

//! High-level intermediate representation (HIR) for Cypher queries.
//!
//! The HIR is a scope-resolved, normalised representation that sits between
//! the typed AST ([`crate::ast`]) and a hypothetical execution engine. It is
//! produced by the [`lower`] module.
//!
//! # Structure
//!
//! | Sub-module | Contents |
//! |---|---|
//! | [`arena`] | Arena allocators and interners for HIR IDs |
//! | [`binding`] | Scope and binding (variable) types |
//! | [`diagnostic`] | Semantic diagnostics produced during lowering |
//! | [`expr`] | HIR expression nodes (`HirExpr`, `ExprKind`, …) |
//! | [`lower`] | The AST → HIR lowering pass |
//! | [`ops`] | Pipeline operation types (`MatchOp`, `ProjectOp`, …) |
//! | [`pattern`] | Normalised graph pattern types |
//!
//! # Relationship to the AST
//!
//! The HIR differs from the AST in the following ways:
//! - Variables are replaced by compact arena [`BindingId`]s.
//! - Scopes are explicit and allocated in [`HirArenas`].
//! - Graph patterns are flattened into node/relationship lists.
//! - Queries are decomposed into a sequence of [`QueryPart`] pipelines.

pub mod arena;
pub mod binding;
pub mod diagnostic;
pub mod expr;
pub mod lower;
pub mod ops;
pub mod pattern;

pub use arena::{BindingId, ExprId, HirArenas, ScopeId};
pub use binding::{Binding, BindingKind, Scope};
pub use diagnostic::HirDiagnostic;
pub use expr::{ExprKind, HirExpr};
pub use ops::{Operation, QueryPart};
pub use pattern::{
    GraphPattern, NodePattern, RelationshipDirection, RelationshipLength, RelationshipPattern,
};

/// A fully lowered and scope-resolved HIR query.
///
/// Produced by [`lower::lower`] from a parsed [`crate::ast::query::Query`].
/// Contains all arenas and the pipeline of query parts.
///
/// > **Note:** `lower()` currently returns `Err(Diagnostics)` whenever any
/// > diagnostic is produced, so successful `HirQuery` values always have an
/// > empty `diagnostics` vector. Non-fatal warning support may be added in a
/// > future release.
#[derive(Debug, Clone)]
pub struct HirQuery {
    /// Arena storage for scopes, bindings, and expressions.
    pub arenas: HirArenas,
    /// The sequence of query parts (pipeline stages).
    pub parts: Vec<QueryPart>,
    /// Reserved for future non-fatal semantic diagnostics.
    ///
    /// Currently always empty: `lower()` returns `Err` on the first
    /// diagnostic rather than accumulating warnings here.
    pub diagnostics: Vec<HirDiagnostic>,
}
