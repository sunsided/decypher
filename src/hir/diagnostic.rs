use crate::error::{CypherError, ErrorKind, Span};

/// A semantic diagnostic produced during HIR lowering.
#[derive(Debug, Clone)]
pub enum HirDiagnostic {
    UnknownVariable {
        name: String,
        span: Span,
    },
    DuplicateVariable {
        name: String,
        first: Span,
        second: Span,
    },
    VariableOutOfScope {
        name: String,
        span: Span,
    },
    InvalidAggregation {
        span: Span,
    },
    InvalidDeleteTarget {
        span: Span,
    },
    InvalidRelationshipPattern {
        span: Span,
    },
    UnknownFunction {
        name: String,
        span: Span,
    },
    UnsupportedFeature {
        feature: String,
        span: Span,
    },
    UnionColumnMismatch {
        span: Span,
    },
    InvalidMergePattern {
        span: Span,
    },
}

impl HirDiagnostic {
    pub fn into_error(self, source: Option<std::sync::Arc<str>>) -> CypherError {
        CypherError {
            kind: self.to_error_kind(),
            span: self.span(),
            source_label: None,
            notes: Vec::new(),
            source,
        }
    }

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
                production: Box::leak(feature.clone().into_boxed_str()),
            },
            _ => ErrorKind::Internal {
                message: format!("{:?}", self),
            },
        }
    }

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
