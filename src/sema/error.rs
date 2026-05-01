//! SemaError wrapper that maps semantic issues to ErrorKind.

use crate::error::{ErrorKind, Span};

/// A semantic analysis error.
///
/// Wraps the various `ErrorKind` variants that are specific to the
/// semantic pass.
#[derive(Debug, Clone)]
pub enum SemaError {
    /// A variable was referenced but never bound.
    UnresolvedVariable { name: String, span: Span },
    /// A variable was redeclared in the same scope.
    RedeclaredVariable {
        name: String,
        first_span: Span,
        redecl_span: Span,
    },
    /// WITH/RETURN mixes aggregates and non-grouping expressions.
    AggregationMix {
        non_grouping: Vec<String>,
        span: Span,
    },
    /// DISTINCT used outside of a projection body.
    DistinctNotAllowed { span: Span },
    /// Invalid reference (e.g. in ORDER BY after WITH referencing
    /// a variable not in the projection).
    InvalidReference {
        name: String,
        reason: &'static str,
        span: Span,
    },
}

impl SemaError {
    /// Convert to the underlying `ErrorKind`.
    pub fn to_error_kind(&self) -> ErrorKind {
        match self {
            SemaError::UnresolvedVariable { name, .. } => {
                ErrorKind::UnresolvedVariable { name: name.clone() }
            }
            SemaError::RedeclaredVariable {
                name, first_span, ..
            } => ErrorKind::RedeclaredVariable {
                name: name.clone(),
                first_span: *first_span,
            },
            SemaError::AggregationMix { non_grouping, .. } => ErrorKind::AggregationMix {
                non_grouping: non_grouping.clone(),
            },
            SemaError::DistinctNotAllowed { .. } => ErrorKind::DistinctNotAllowed,
            SemaError::InvalidReference { name, reason, .. } => ErrorKind::InvalidReference {
                name: name.clone(),
                reason,
            },
        }
    }
}
