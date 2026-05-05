//! Semantic analysis for openCypher queries.
//!
//! The semantic analyzer performs name resolution, scope tracking, and
//! aggregation-rule validation over a parsed [`Query`] AST.
//!
//! # Phases
//!
//! 1. **Name resolution** ([`resolve_names`]) — walks the query and verifies
//!    that every variable reference is bound in the visible scope. Reports
//!    [`SemaError::UnresolvedVariable`] and [`SemaError::RedeclaredVariable`]
//!    diagnostics.
//!
//! 2. **Aggregation validation** ([`check_aggregation`]) — checks that `WITH`
//!    and `RETURN` projections do not mix aggregate and non-aggregate
//!    (non-grouping) expressions, and that `DISTINCT` is used in valid
//!    positions.

mod aggregation;
mod error;
mod resolve;
mod scope;

pub use aggregation::{AggregationViolation, check_aggregation};
pub use error::SemaError;
pub use resolve::{ResolutionResult, resolve_names};
pub use scope::{ScopeStack, SymbolKind};

use crate::ast::query::Query;
use crate::error::Diagnostics;

/// Perform full semantic analysis on `query`.
///
/// Runs all semantic analysis phases in order:
/// 1. Name resolution
/// 2. Aggregation validation
///
/// Returns `Ok(())` when no errors were found, or `Err(Diagnostics)` with
/// all semantic issues discovered.
///
/// # Example
///
/// ```
/// use cypher::{parse, sema};
///
/// let query = parse("MATCH (n) RETURN n").unwrap();
/// assert!(sema::analyze(&query).is_ok());
/// ```
pub fn analyze(query: &Query) -> Result<(), Diagnostics> {
    let mut diagnostics = Diagnostics { errors: Vec::new() };

    // Phase 1: Name resolution
    if let Err(errs) = resolve_names(query) {
        diagnostics.errors.extend(errs);
    }

    // Phase 2: Aggregation rules
    check_aggregation(query, &mut diagnostics);

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

/// Perform full semantic analysis on `query`, returning all diagnostics.
///
/// Unlike [`analyze`], this function always succeeds and returns the
/// diagnostics as a [`Diagnostics`] value rather than a `Result`. The
/// returned [`Diagnostics`] is empty when the query is semantically valid.
///
/// # Example
///
/// ```
/// use cypher::{parse, sema};
///
/// let query = parse("MATCH (n) RETURN n").unwrap();
/// let diags = sema::analyze_all(&query);
/// assert!(diags.is_empty());
/// ```
pub fn analyze_all(query: &Query) -> Diagnostics {
    match analyze(query) {
        Ok(()) => Diagnostics { errors: Vec::new() },
        Err(diags) => diags,
    }
}
