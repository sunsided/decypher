//! Integration tests for variable-redeclaration semantic errors.
//!
//! These tests verify that the semantic analyzer detects and reports
//! `RedeclaredVariable` errors when the same name is introduced more than
//! once in the same scope.

use cypher::sema::analyze;
use cypher::{ErrorKind, parse};

/// Two node patterns in the same `MATCH` that bind the same variable produce
/// a `RedeclaredVariable` error.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (a), (a) RETURN a` — `a` is bound twice in one scope.
/// Expectation: Analysis returns `Err` with one `RedeclaredVariable { name: "a" }`.
#[test]
fn redeclare_in_match_pattern() {
    let query = parse("MATCH (a), (a) RETURN a").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "a");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// Two consecutive `UNWIND` clauses binding the same variable produce a
/// `RedeclaredVariable` error.
///
/// Unit: `sema::analyze()`
/// Precondition: `UNWIND [1] AS x UNWIND [2] AS x RETURN x` — `x` bound twice.
/// Expectation: Analysis returns `Err` with one `RedeclaredVariable { name: "x" }`.
#[test]
fn redeclare_in_unwind() {
    let query = parse("UNWIND [1] AS x UNWIND [2] AS x RETURN x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// Two `AS x` aliases in the same `WITH` clause produce a `RedeclaredVariable` error.
///
/// Unit: `sema::analyze()`
/// Precondition: `WITH n AS x, n.name AS x RETURN x` — alias `x` duplicated.
/// Expectation: Analysis returns `Err` with one `RedeclaredVariable { name: "x" }`.
#[test]
fn redeclare_in_with_alias() {
    let query = parse("MATCH (n) WITH n AS x, n.name AS x RETURN x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// Two `AS x` aliases in the same `RETURN` clause produce a `RedeclaredVariable` error.
///
/// Unit: `sema::analyze()`
/// Precondition: `RETURN n AS x, n.name AS x` — alias `x` duplicated in RETURN.
/// Expectation: Analysis returns `Err` with one `RedeclaredVariable { name: "x" }`.
#[test]
fn redeclare_in_return_alias() {
    let query = parse("MATCH (n) RETURN n AS x, n.name AS x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

/// Two separate `FOREACH` statements each introduce an independent scope.
///
/// Unit: `sema::analyze()`
/// Precondition: Two `FOREACH (x IN … | …)` statements reusing variable `x`.
///   Each FOREACH introduces its own scope, so reusing `x` is valid.
/// Expectation: Analysis returns `Ok`.
#[test]
fn redeclare_in_foreach() {
    let query = parse("FOREACH (x IN [1, 2, 3] | CREATE (:Label {val: x})) FOREACH (x IN [4, 5] | CREATE (:Other {val: x}))").expect("should parse");
    let result = analyze(&query);
    // This should be OK — each FOREACH has its own scope
    assert!(result.is_ok());
}

/// Re-using a variable name across a `WITH` boundary (shadowing) is allowed.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (x) WITH x.name AS x RETURN x` — `x` in MATCH is a node,
///   `x` after WITH is a string alias. The WITH introduces a new scope.
/// Expectation: Analysis returns `Ok`.
#[test]
fn shadowing_across_scopes_allowed() {
    // WITH replaces the visible scope, so reusing the same name in the
    // projected scope should be allowed.
    let query = parse("MATCH (x) WITH x.name AS x RETURN x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}

/// A node variable reused at both ends of a relationship chain in the same
/// MATCH pattern produces a `RedeclaredVariable` error.
///
/// Unit: `sema::analyze()`
/// Precondition: `MATCH (a)-[r]->(a) RETURN a, r` — `a` bound twice in one pattern.
/// Expectation: Analysis returns `Err` with one `RedeclaredVariable { name: "a" }`.
#[test]
fn redeclare_in_pattern_chain() {
    let query = parse("MATCH (a)-[r]->(a) RETURN a, r").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "a");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}
