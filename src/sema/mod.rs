//! Semantic analysis for openCypher queries.
//!
//! The semantic analyzer performs name resolution, scope tracking, and
//! aggregation-rule validation over a parsed [`Query`] AST.

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

/// Analyze a query for semantic errors.
///
/// Returns `Ok(())` if no errors were found, or a `Diagnostics` with all
/// semantic issues discovered.
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

/// Analyze a query, always returning the full diagnostics set.
pub fn analyze_all(query: &Query) -> Diagnostics {
    match analyze(query) {
        Ok(()) => Diagnostics { errors: Vec::new() },
        Err(diags) => diags,
    }
}
