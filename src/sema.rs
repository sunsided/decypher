//! Semantic analysis for Cypher queries.
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
//! 2. **Aggregation validation** ([`check_aggregation`]) — checks that the
//!    aggregating items of `WITH` and `RETURN` projections use no variables or
//!    properties outside their aggregates that are not grouping keys
//!    (`RETURN n.a + count(*)`). [`check_aggregation_with`] and
//!    [`analyze_with`] take a [`Catalog`] for user-defined
//!    aggregates.

mod aggregation;
mod catalog;
mod error;
mod resolve;
mod scope;
mod structure;
mod types;

pub use aggregation::{AggregationViolation, check_aggregation, check_aggregation_with};
#[cfg(feature = "hir")]
pub(crate) use aggregation::{
    implicit_grouping_keys, implicit_grouping_keys_in_order, misplaced_aggregates,
};
pub use catalog::{Catalog, Signature};
pub use error::SemaError;
pub use resolve::{ResolutionResult, resolve_names, resolve_names_with};
pub use scope::{ScopeStack, SymbolKind};
#[cfg(feature = "hir")]
pub(crate) use structure::{StructuralError, structural_errors};

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
/// use decypher::{parse, sema};
///
/// let query = parse("MATCH (n) RETURN n").unwrap();
/// assert!(sema::analyze(&query).is_ok());
/// ```
pub fn analyze(query: &Query) -> Result<(), Diagnostics> {
    analyze_with(query, &Catalog::default())
}

/// Perform full semantic analysis on `query`, checking calls against
/// `catalog`.
///
/// Identical to [`analyze`] except for user-defined or plugin functions and
/// procedures, or a [strict](Catalog::set_strict) catalog.
///
/// # Example
///
/// ```
/// use decypher::{parse, sema};
///
/// let query = parse("MATCH (n) RETURN n.k, apoc.agg.median(n.v) AS m").unwrap();
/// let mut catalog = sema::Catalog::default();
/// catalog.register_aggregate("apoc.agg.median");
/// assert!(sema::analyze_with(&query, &catalog).is_ok());
/// ```
pub fn analyze_with(query: &Query, catalog: &Catalog) -> Result<(), Diagnostics> {
    let mut diagnostics = Diagnostics { errors: Vec::new() };

    // Phase 1: Name resolution
    if let Err(errs) = resolve_names_with(query, catalog) {
        diagnostics.errors.extend(errs);
    }

    // Phase 2: Aggregation rules
    check_aggregation_with(query, catalog, &mut diagnostics);

    // Phase 3: Structural rules
    for error in structure::structural_errors(query, catalog) {
        diagnostics.errors.push(crate::error::CypherError {
            kind: error.kind(),
            span: error.span(),
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    }

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
/// use decypher::{parse, sema};
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
