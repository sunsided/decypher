//! Integration tests for `WITH` clause scope semantics.
//!
//! These tests verify that the semantic analyzer correctly enforces scope
//! boundaries introduced by `WITH` clauses: only variables explicitly
//! projected by `WITH` remain visible in subsequent clauses.

use cypher::sema::analyze;
use cypher::{ErrorKind, parse};

/// Variables not projected by a `WITH` clause are no longer visible after it.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (n) WITH n.name AS name WHERE n.other IS NOT NULL RETURN name`.
///   `n` is not re-projected, so the WHERE clause must not reference it.
/// Expectation: Analysis returns `Err` with one `UnresolvedVariable { name: "n" }`.
#[test]
fn with_hides_previous_variables() {
    // After WITH n.name AS name, 'n' should be out of scope
    let query = parse("MATCH (n) WITH n.name AS name WHERE n.other IS NOT NULL RETURN name")
        .expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    // Should have an unresolved variable error for 'n' in the WHERE clause
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::UnresolvedVariable { name } => {
            assert_eq!(name, "n");
        }
        _ => panic!(
            "expected UnresolvedVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// A property lookup without `AS alias` does not introduce a bound name.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (n) WITH n.name RETURN name` — `name` is never explicitly bound.
/// Expectation: Analysis returns `Err` with one `UnresolvedVariable { name: "name" }`.
#[test]
fn with_unaliased_property_projection_requires_alias() {
    // MATCH (n) WITH n.name RETURN name — property lookups do not bind `name`
    // unless they are explicitly aliased.
    let query = parse("MATCH (n) WITH n.name RETURN name").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::UnresolvedVariable { name } => {
            assert_eq!(name, "name");
        }
        _ => panic!(
            "expected UnresolvedVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// `WITH n` (bare variable projection without alias) re-binds `n`.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (n) WITH n RETURN n` — variable projected without alias.
/// Expectation: Analysis returns `Ok`.
#[test]
fn with_variable_projection_binds() {
    // MATCH (n) WITH n RETURN n — unaliased variable projection binds as 'n'
    let query = parse("MATCH (n) WITH n RETURN n").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}

/// Multiple `WITH` clauses chain scopes correctly across query parts.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (a) WITH a.x AS x MATCH (b) WITH x, b.y AS y RETURN x, y`.
///   All referenced names are properly projected by their respective WITH clauses.
/// Expectation: Analysis returns `Ok`.
#[test]
fn with_multiple_parts() {
    // Multi-part query: each WITH introduces a new scope
    let query = parse("MATCH (a) WITH a.x AS x MATCH (b) WITH x, b.y AS y RETURN x, y")
        .expect("should parse");
    let result = analyze(&query);
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}

/// Variables not included in a `WITH` projection cannot be referenced later.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (a), (b) WITH a RETURN b` — `b` is excluded from the WITH.
/// Expectation: Analysis returns `Err` with one `UnresolvedVariable { name: "b" }`.
#[test]
fn with_scope_prevents_access_to_earlier_vars() {
    // MATCH (a), (b) WITH a RETURN b — 'b' should be out of scope after WITH
    let query = parse("MATCH (a), (b) WITH a RETURN b").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::UnresolvedVariable { name } => {
            assert_eq!(name, "b");
        }
        _ => panic!(
            "expected UnresolvedVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// `WITH *` preserves all currently visible bindings.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (n) WITH * RETURN n` — `*` re-projects everything.
/// Expectation: Analysis returns `Ok`.
#[test]
fn with_star_projection() {
    // WITH * should preserve the current visible bindings.
    let query = parse("MATCH (n) WITH * RETURN n").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}
