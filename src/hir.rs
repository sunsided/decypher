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

/// A fully resolved HIR query.
#[derive(Debug, Clone)]
pub struct HirQuery {
    pub arenas: HirArenas,
    pub parts: Vec<QueryPart>,
    pub diagnostics: Vec<HirDiagnostic>,
}
