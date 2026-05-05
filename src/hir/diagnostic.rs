//! Semantic diagnostics produced during HIR lowering.
//!
//! [`HirDiagnostic`] variants capture every semantic issue that the lowering
//! pass detects. They can be converted into [`crate::error::CypherError`]
//! for use in public error reporting.

use crate::error::{CypherError, ErrorKind, Span};

/// A semantic diagnostic produced during HIR lowering.
///
/// Each variant corresponds to a distinct semantic error category. All
/// variants carry at least a [`Span`] pointing to the problematic source
/// location.
#[derive(Debug, Clone)]
pub enum HirDiagnostic {
    /// A referenced variable has no binding in the visible scope.
    UnknownVariable {
        /// The unresolved variable name.
        name: String,
        /// Source location of the reference.
        span: Span,
    },
    /// A variable was introduced more than once in the same scope.
    DuplicateVariable {
        /// The variable name.
        name: String,
        /// Location of the first introduction.
        first: Span,
        /// Location of the duplicate introduction.
        second: Span,
    },
    /// A variable was used outside the scope in which it was introduced.
    VariableOutOfScope {
        /// The variable name.
        name: String,
        /// Source location of the out-of-scope reference.
        span: Span,
    },
    /// An aggregation was used in a context that does not allow it.
    InvalidAggregation {
        /// Source location of the invalid aggregation expression.
        span: Span,
    },
    /// A `DELETE` target expression is not a valid node or relationship reference.
    InvalidDeleteTarget {
        /// Source location of the invalid target.
        span: Span,
    },
    /// A relationship pattern is structurally invalid.
    InvalidRelationshipPattern {
        /// Source location of the invalid pattern.
        span: Span,
    },
    /// A called function is not known to the lowering pass.
    UnknownFunction {
        /// The function name.
        name: String,
        /// Source location of the call.
        span: Span,
    },
    /// A language feature is not yet supported by the HIR lowering pass.
    UnsupportedFeature {
        /// A short description of the unsupported feature.
        feature: String,
        /// Source location of the unsupported construct.
        span: Span,
    },
    /// Two `UNION` branches project a different number of columns.
    UnionColumnMismatch {
        /// Source location of the mismatched `UNION`.
        span: Span,
    },
    /// A `MERGE` pattern is structurally invalid.
    InvalidMergePattern {
        /// Source location of the invalid merge pattern.
        span: Span,
    },
}

impl HirDiagnostic {
    /// Convert this diagnostic into a [`CypherError`], attaching the given
    /// `source` text if provided.
    pub fn into_error(self, source: Option<std::sync::Arc<str>>) -> CypherError {
        CypherError {
            kind: self.to_error_kind(),
            span: self.span(),
            source_label: None,
            notes: Vec::new(),
            source,
        }
    }

    /// Map this diagnostic to the corresponding [`ErrorKind`].
    fn to_error_kind(&self) -> ErrorKind {
        match self {
            HirDiagnostic::UnknownVariable { name, .. } => {
                ErrorKind::UnresolvedVariable { name: name.clone() }
            }
            HirDiagnostic::DuplicateVariable { name, first, .. } => ErrorKind::RedeclaredVariable {
                name: name.clone(),
                first_span: *first,
            },
            HirDiagnostic::InvalidAggregation { .. } => ErrorKind::AggregationMix {
                non_grouping: vec![],
            },
            HirDiagnostic::UnsupportedFeature { feature, .. } => ErrorKind::Unsupported {
                production: std::borrow::Cow::Owned(feature.clone()),
            },
            _ => ErrorKind::Internal {
                message: format!("{:?}", self),
            },
        }
    }

    /// Extract the primary [`Span`] from this diagnostic.
    fn span(&self) -> Span {
        match self {
            HirDiagnostic::UnknownVariable { span, .. } => *span,
            HirDiagnostic::DuplicateVariable { second: span, .. } => *span,
            HirDiagnostic::VariableOutOfScope { span, .. } => *span,
            HirDiagnostic::InvalidAggregation { span } => *span,
            HirDiagnostic::InvalidDeleteTarget { span } => *span,
            HirDiagnostic::InvalidRelationshipPattern { span } => *span,
            HirDiagnostic::UnknownFunction { span, .. } => *span,
            HirDiagnostic::UnsupportedFeature { span, .. } => *span,
            HirDiagnostic::UnionColumnMismatch { span } => *span,
            HirDiagnostic::InvalidMergePattern { span } => *span,
        }
    }
}
