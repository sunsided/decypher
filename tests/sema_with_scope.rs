//! Integration tests for `WITH` clause scope semantics.
//!
//! These tests verify that the semantic analyzer correctly enforces scope
//! boundaries introduced by `WITH` clauses: only variables explicitly
//! projected by `WITH` remain visible in subsequent clauses.

use decypher::sema::analyze;
use decypher::{ErrorKind, parse};

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

/// Subqueries see the enclosing scope and keep it intact; `CALL` exports its
/// returned columns, and every UNION branch starts from the same scope (#67).
///
/// Unit: `sema::analyze()`
/// Precondition: Correlated CALL subqueries, a UNION inside and outside a
///               subquery, and COUNT/EXISTS bodies containing `WITH` or new
///               pattern variables.
/// Expectation: Valid queries pass; variables that are local to a subquery
///              or its pattern are unresolved outside it.
#[test]
fn subqueries_import_export_and_keep_outer_scope() {
    for q in [
        "CALL { MATCH (n) RETURN n } RETURN n",
        "WITH 1 AS x CALL { WITH x RETURN x + 1 AS y } RETURN x, y",
        "MATCH (n) CALL { WITH n MATCH (n)-->(m) RETURN count(m) AS c } RETURN n, c",
        "CALL { RETURN 1 AS a UNION RETURN 2 AS a } RETURN a",
        "RETURN 1 AS a UNION RETURN 2 AS a",
        "MATCH (n) RETURN COUNT { MATCH (n)-->(m) WITH m RETURN m } AS c, n",
        "MATCH (n) WHERE EXISTS { (n)-->(m) WHERE m.x > 1 } RETURN n",
    ] {
        let query = parse(q).expect("should parse");
        let result = analyze(&query);
        assert!(result.is_ok(), "query: {q}, analysis failed: {result:?}");
    }
    for (q, var) in [
        ("MATCH (n) CALL { WITH n CREATE (m) } RETURN n, m", "m"),
        ("MATCH (n) WHERE EXISTS { (n)-->(m) } RETURN m", "m"),
        ("CALL { RETURN zz AS a } RETURN a", "zz"),
    ] {
        let query = parse(q).expect("should parse");
        let errs = analyze(&query).unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e.kind(),
                ErrorKind::UnresolvedVariable { name } if name == var
            )),
            "query: {q}, got: {errs:?}"
        );
    }
}

/// A `CALL` subquery exports only aliased or plain-variable columns, and a
/// duplicate alias inside the subquery is reported once, not again at the
/// `CALL` site.
///
/// Unit: `sema::analyze()`
/// Precondition: An unaliased property item, and `RETURN 1 AS a, 2 AS a`
///               inside a subquery.
/// Expectation: The property item exports no `name`; the duplicate alias
///              yields exactly one `RedeclaredVariable`.
#[test]
fn subquery_exports_aliases_and_variables_only() {
    let query = parse("MATCH (n) CALL { WITH n RETURN n.name } RETURN name").expect("should parse");
    assert!(analyze(&query).is_err());

    let query = parse("CALL { RETURN 1 AS a, 2 AS a } RETURN a").expect("should parse");
    let errs = analyze(&query).unwrap_err();
    assert_eq!(errs.len(), 1, "got: {errs:?}");
    assert!(matches!(
        errs.errors[0].kind(),
        ErrorKind::RedeclaredVariable { name, .. } if name == "a"
    ));
}

/// Projection columns: duplicates are reported whether aliased or not, and
/// aliases are not visible to other items of the same projection, so a
/// `RETURN` alias may shadow a variable of the query (TCK queries such as
/// `MATCH (a) RETURN a.id IS NOT NULL AS a, a IS NOT NULL AS b`).
///
/// Unit: `sema::analyze()`
/// Precondition: Duplicate aliased and unaliased columns; aliases shadowing
///               and referenced within the same projection; ORDER BY on an
///               alias.
/// Expectation: Duplicates and same-projection alias references fail;
///              shadowing and ORDER BY on aliases pass.
#[test]
fn projection_columns_and_alias_visibility() {
    for q in [
        "MATCH (a) RETURN a.id IS NOT NULL AS a, a IS NOT NULL AS b",
        "MATCH (a) RETURN a.id AS a, a.id",
        "MATCH (n) RETURN n.x AS k ORDER BY k",
    ] {
        let query = parse(q).expect("should parse");
        let result = analyze(&query);
        assert!(result.is_ok(), "query: {q}, analysis failed: {result:?}");
    }
    for q in [
        "MATCH (n) RETURN count(*), count(*)",
        "MATCH (n) RETURN n, n",
        "RETURN 1 AS a, 2 AS a",
        "WITH 1 AS a, a + 1 AS b RETURN b",
    ] {
        let query = parse(q).expect("should parse");
        let errs = analyze(&query).unwrap_err();
        assert_eq!(errs.len(), 1, "query: {q}, got: {errs:?}");
    }
}
