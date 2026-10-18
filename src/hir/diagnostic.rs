//! Semantic diagnostics produced during HIR lowering.
//!
//! [`HirDiagnostic`] variants capture every semantic issue that the lowering
//! pass detects. They can be converted into [`crate::error::CypherError`]
//! for use in public error reporting.

use std::borrow::Cow;

use crate::error::{CypherError, ErrorKind, Span, ValueTypes, VariableKind};

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
    /// A projection item combines an aggregate with variables or properties
    /// that are not grouping keys (`RETURN n.a + count(*)`).
    InvalidAggregation {
        /// The offending references, as printed Cypher (`n.a`).
        non_grouping: Vec<String>,
        /// Source location of the invalid aggregation expression.
        span: Span,
    },
    /// An aggregate called where aggregation is not allowed
    /// (`MATCH … WHERE count(n) > 1`, `[x IN list | count(*)]`).
    AggregateNotAllowed {
        /// The dot-joined name of the aggregate function.
        function: String,
        /// Source location of the call.
        span: Span,
    },
    /// A `DELETE` target is not a node, relationship or path
    /// (`DELETE n:Person`).
    InvalidDeleteTarget {
        /// Source location of the invalid target.
        span: Span,
    },
    /// A relationship pattern its clause cannot use (an untyped or
    /// undirected relationship in `CREATE`, a relationship matched twice).
    InvalidRelationshipPattern {
        /// Why the pattern is invalid.
        reason: &'static str,
        /// Source location of the invalid pattern.
        span: Span,
    },
    /// Clauses that cannot be combined (`UNION` with `UNION ALL`, updates in
    /// an `EXISTS` subquery).
    InvalidClauseComposition {
        /// Why the clauses cannot be combined.
        reason: &'static str,
        /// Source location of the offending clause.
        span: Span,
    },
    /// A `WITH` item other than a variable has no `AS` alias.
    MissingAlias {
        /// Source location of the item.
        span: Span,
    },
    /// `RETURN *` with no variables in scope.
    NoVariablesInScope {
        /// Source location of the clause.
        span: Span,
    },
    /// A `SKIP` or `LIMIT` value that is not a constant, non-negative
    /// integer (`SKIP n.count`, `LIMIT -1`).
    InvalidSkipLimit {
        /// Why the value is invalid.
        reason: &'static str,
        /// Source location of the value.
        span: Span,
    },
    /// A parameter used as the property map of a `MATCH` or `MERGE` pattern
    /// (`MATCH (n $param)`).
    InvalidParameterUse {
        /// Source location of the parameter.
        span: Span,
    },
    /// A non-deterministic function inside an aggregate (`count(rand())`).
    NonDeterministicAggregate {
        /// Source location of the call.
        span: Span,
    },
    /// A pattern expression used as a value (`RETURN (n)-->()`).
    PatternExpressionOutsidePredicate {
        /// Source location of the pattern.
        span: Span,
    },
    /// A variable used as two kinds of entity
    /// (`MATCH (r) MATCH ()-[r]-()`).
    VariableTypeConflict {
        /// The variable name.
        name: String,
        /// What the variable was first used as (`a node`).
        bound_as: VariableKind,
        /// What it is used as here (`a relationship`).
        used_as: VariableKind,
        /// Source location of the conflicting use.
        span: Span,
    },
    /// A function a strict [`crate::sema::Catalog`] does not know.
    UnknownFunction {
        /// The dot-joined function name.
        name: String,
        /// Source location of the call.
        span: Span,
    },
    /// A procedure a strict [`crate::sema::Catalog`] does not know.
    ProcedureNotFound {
        /// The dot-joined procedure name.
        name: String,
        /// Source location of the call.
        span: Span,
    },
    /// A value of a type its operator, clause or function does not accept
    /// (`NOT 'a'`, `labels(p)` on a path).
    InvalidArgumentType {
        /// What takes the value: `NOT`, ``argument 1 of `labels()` ``.
        context: Cow<'static, str>,
        /// The types accepted there.
        expected: ValueTypes,
        /// The types the value can have.
        found: ValueTypes,
        /// Source location of the value.
        span: Span,
    },
    /// A call with too few or too many arguments.
    InvalidNumberOfArguments {
        /// The dot-joined function or procedure name.
        name: String,
        /// The fewest arguments accepted.
        min: u16,
        /// The most arguments accepted, or `None` when unbounded.
        max: Option<u16>,
        /// The number of arguments passed.
        found: u16,
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
    /// Two `UNION` branches return different columns.
    UnionColumnMismatch {
        /// Source location of the mismatched `UNION`.
        span: Span,
    },
}

impl From<crate::sema::StructuralError> for HirDiagnostic {
    fn from(error: crate::sema::StructuralError) -> Self {
        use crate::sema::StructuralError as E;
        match error {
            E::UnionColumnMismatch { span } => HirDiagnostic::UnionColumnMismatch { span },
            E::InvalidClauseComposition { reason, span } => {
                HirDiagnostic::InvalidClauseComposition { reason, span }
            }
            E::InvalidRelationshipPattern { reason, span } => {
                HirDiagnostic::InvalidRelationshipPattern { reason, span }
            }
            E::InvalidDeleteTarget { span } => HirDiagnostic::InvalidDeleteTarget { span },
            E::MissingAlias { span } => HirDiagnostic::MissingAlias { span },
            E::InvalidSkipLimit { reason, span } => {
                HirDiagnostic::InvalidSkipLimit { reason, span }
            }
            E::InvalidParameterUse { span } => HirDiagnostic::InvalidParameterUse { span },
            E::NonDeterministicAggregate { span } => {
                HirDiagnostic::NonDeterministicAggregate { span }
            }
            E::PatternExpressionOutsidePredicate { span } => {
                HirDiagnostic::PatternExpressionOutsidePredicate { span }
            }
            E::InvalidArgumentType {
                context,
                expected,
                found,
                span,
            } => HirDiagnostic::InvalidArgumentType {
                context,
                expected,
                found,
                span,
            },
            E::InvalidNumberOfArguments {
                name,
                min,
                max,
                found,
                span,
            } => HirDiagnostic::InvalidNumberOfArguments {
                name,
                min,
                max,
                found,
                span,
            },
            E::UnknownFunction { name, span } => HirDiagnostic::UnknownFunction { name, span },
            E::ProcedureNotFound { name, span } => HirDiagnostic::ProcedureNotFound { name, span },
            E::VariableTypeConflict {
                name,
                bound_as,
                used_as,
                span,
            } => HirDiagnostic::VariableTypeConflict {
                name,
                bound_as,
                used_as,
                span,
            },
        }
    }
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
            HirDiagnostic::InvalidAggregation { non_grouping, .. } => ErrorKind::AggregationMix {
                non_grouping: non_grouping.clone(),
            },
            HirDiagnostic::AggregateNotAllowed { function, .. } => ErrorKind::AggregateNotAllowed {
                function: function.clone(),
            },
            HirDiagnostic::UnsupportedFeature { feature, .. } => ErrorKind::Unsupported {
                production: std::borrow::Cow::Owned(feature.clone()),
            },
            HirDiagnostic::InvalidDeleteTarget { .. } => ErrorKind::InvalidDeleteTarget,
            HirDiagnostic::InvalidRelationshipPattern { reason, .. } => {
                ErrorKind::InvalidRelationshipPattern { reason }
            }
            HirDiagnostic::InvalidClauseComposition { reason, .. } => {
                ErrorKind::InvalidClauseComposition { reason }
            }
            HirDiagnostic::MissingAlias { .. } => ErrorKind::MissingAlias,
            HirDiagnostic::NoVariablesInScope { .. } => ErrorKind::NoVariablesInScope,
            HirDiagnostic::InvalidSkipLimit { reason, .. } => {
                ErrorKind::InvalidSkipLimit { reason }
            }
            HirDiagnostic::InvalidParameterUse { .. } => ErrorKind::InvalidParameterUse,
            HirDiagnostic::NonDeterministicAggregate { .. } => ErrorKind::NonDeterministicAggregate,
            HirDiagnostic::PatternExpressionOutsidePredicate { .. } => {
                ErrorKind::PatternExpressionOutsidePredicate
            }
            HirDiagnostic::VariableTypeConflict {
                name,
                bound_as,
                used_as,
                ..
            } => ErrorKind::VariableTypeConflict {
                name: name.clone(),
                bound_as: *bound_as,
                used_as: *used_as,
            },
            HirDiagnostic::UnionColumnMismatch { .. } => ErrorKind::UnionColumnMismatch,
            HirDiagnostic::UnknownFunction { name, .. } => {
                ErrorKind::UnknownFunction { name: name.clone() }
            }
            HirDiagnostic::ProcedureNotFound { name, .. } => {
                ErrorKind::ProcedureNotFound { name: name.clone() }
            }
            HirDiagnostic::InvalidArgumentType {
                context,
                expected,
                found,
                ..
            } => ErrorKind::InvalidArgumentType {
                context: context.clone(),
                expected: *expected,
                found: *found,
            },
            HirDiagnostic::InvalidNumberOfArguments {
                name,
                min,
                max,
                found,
                ..
            } => ErrorKind::InvalidNumberOfArguments {
                name: name.clone(),
                min: *min,
                max: *max,
                found: *found,
            },
        }
    }

    /// Extract the primary [`Span`] from this diagnostic.
    fn span(&self) -> Span {
        match self {
            HirDiagnostic::UnknownVariable { span, .. } => *span,
            HirDiagnostic::DuplicateVariable { second: span, .. } => *span,
            HirDiagnostic::InvalidAggregation { span, .. } => *span,
            HirDiagnostic::InvalidDeleteTarget { span } => *span,
            HirDiagnostic::InvalidRelationshipPattern { span, .. } => *span,
            HirDiagnostic::InvalidClauseComposition { span, .. } => *span,
            HirDiagnostic::MissingAlias { span } => *span,
            HirDiagnostic::NoVariablesInScope { span } => *span,
            HirDiagnostic::InvalidSkipLimit { span, .. } => *span,
            HirDiagnostic::InvalidParameterUse { span } => *span,
            HirDiagnostic::NonDeterministicAggregate { span } => *span,
            HirDiagnostic::PatternExpressionOutsidePredicate { span } => *span,
            HirDiagnostic::VariableTypeConflict { span, .. } => *span,
            HirDiagnostic::UnknownFunction { span, .. } => *span,
            HirDiagnostic::ProcedureNotFound { span, .. } => *span,
            HirDiagnostic::InvalidArgumentType { span, .. } => *span,
            HirDiagnostic::InvalidNumberOfArguments { span, .. } => *span,
            HirDiagnostic::UnsupportedFeature { span, .. } => *span,
            HirDiagnostic::UnionColumnMismatch { span } => *span,
            HirDiagnostic::AggregateNotAllowed { span, .. } => *span,
        }
    }
}
