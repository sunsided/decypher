//! Integration tests for the aggregation rules of `sema::check_aggregation`.
//!
//! Every non-aggregate projection item is a grouping key. An aggregating item
//! may only use, outside its aggregates, grouping keys that are variables or
//! property accesses (#97).

use decypher::sema::{Catalog, analyze, analyze_with};
use decypher::{ErrorKind, parse};

/// Returns the non-grouping references `sema::analyze` reports for `query`.
fn non_grouping(query: &str, catalog: &Catalog) -> Vec<String> {
    let parsed = parse(query).expect("should parse");
    match analyze_with(&parsed, catalog) {
        Ok(()) => Vec::new(),
        Err(diagnostics) => diagnostics
            .errors
            .iter()
            .flat_map(|e| match e.kind() {
                ErrorKind::AggregationMix { non_grouping } => non_grouping.clone(),
                other => panic!("query: {query}, unexpected error: {other}"),
            })
            .collect(),
    }
}

/// Non-aggregate items group implicitly, whatever their expression.
///
/// Unit: `sema::analyze()`
/// Precondition: Computed, aliased, literal and parameter items next to an
///               aggregate; grouping keys used inside aggregating items.
/// Expectation: Analysis succeeds.
#[test]
fn computed_grouping_keys_are_accepted() {
    for query in [
        "MATCH (n) WITH toUpper(n.name) AS k, count(*) AS c RETURN k, c",
        "MATCH (n) RETURN n.a + n.b AS k, count(*) AS c",
        "MATCH (n) RETURN 1 AS one, $p AS p, count(*) AS c",
        "MATCH (n) RETURN n, n.x + count(n) AS y",
        "MATCH (n) RETURN n.x, n.x + count(n) AS y",
        "MATCH (n)-->(m) RETURN n, m.k, m.k * count(*) AS y",
        "MATCH (n) RETURN [x IN collect(n) | x.name] AS names",
        "MATCH (n) RETURN reduce(s = 0, v IN collect(n.v) | s + v) AS total",
        "MATCH (n) WITH n.k AS k, count(*) AS c WITH k, c, k + max(c) AS m RETURN m",
        "MATCH (n) RETURN count(*) + 1 AS c, $p * sum(n.x) AS s",
    ] {
        let found = non_grouping(query, &Catalog::default());
        assert!(found.is_empty(), "query: {query}, got: {found:?}");
    }
}

/// References that are not grouping keys on their own are implicit grouping
/// keys and rejected, as in Neo4j (openCypher TCK `Return6`, `With6`).
///
/// Unit: `sema::analyze()`
/// Precondition: Aggregating items using variables or properties that are
///               not projected, or only projected inside a larger expression.
/// Expectation: One `AggregationMix` naming the offending references.
#[test]
fn implicit_grouping_keys_are_rejected() {
    for (query, expected) in [
        ("MATCH (n) RETURN n.x + count(n)", vec!["n.x"]),
        (
            "MATCH (me)--(you) RETURN me.age + count(you.age)",
            vec!["me.age"],
        ),
        (
            "MATCH (me)--(you) RETURN me.age + you.age, me.age + you.age + count(*)",
            vec!["me.age", "you.age"],
        ),
        (
            "MATCH (me)--(you) WITH me.age + count(you.age) AS agg RETURN agg",
            vec!["me.age"],
        ),
        ("MATCH (n)-->(m) RETURN n.k, m + count(*)", vec!["m"]),
    ] {
        let found = non_grouping(query, &Catalog::default());
        assert_eq!(found, expected, "query: {query}");
    }
}

/// Functions registered as aggregates count as aggregates, so they are not
/// grouping keys and may take ungrouped arguments.
///
/// Unit: `sema::analyze_with()` / `sema::analyze()`
/// Precondition: `apoc.agg.percentiles` registered, used next to `count(*)`
///               and next to an ungrouped property.
/// Expectation: The first query is valid with and without the registration;
///              the second is rejected only once the function is registered.
#[test]
fn registered_aggregates_are_aggregates() {
    let mut catalog = Catalog::default();
    catalog.register_aggregate("apoc.agg.percentiles");

    let query = "MATCH (n) WITH apoc.agg.percentiles(n.score) AS p, count(*) AS c RETURN p, c";
    assert!(non_grouping(query, &catalog).is_empty());
    assert!(analyze(&parse(query).unwrap()).is_ok());

    let query = "MATCH (n) RETURN n.x + apoc.agg.percentiles(n.score) AS p";
    assert_eq!(non_grouping(query, &catalog), vec!["n.x"]);
    assert!(non_grouping(query, &Catalog::default()).is_empty());
}

/// `WITH *` and `RETURN *` project every variable in scope, so all of them
/// are grouping keys (#102 review).
///
/// Unit: `sema::analyze()`
/// Precondition: `*` next to an aggregating item over a matched variable.
/// Expectation: Analysis succeeds.
#[test]
fn star_variables_are_grouping_keys() {
    for query in [
        "MATCH (n) WITH *, n.x + count(*) AS c RETURN n, c",
        "MATCH (n) RETURN *, n.x + count(*) AS c",
    ] {
        let found = non_grouping(query, &Catalog::default());
        assert!(found.is_empty(), "query: {query}, got: {found:?}");
    }
}

/// The aggregating `ORDER BY` items of an aggregating projection follow the
/// same rule, with the projection's columns counting as grouping keys
/// (openCypher TCK `ReturnOrderBy6`, `WithOrderBy4`).
///
/// Unit: `sema::analyze()`
/// Precondition: `ORDER BY` repeating a complex grouping expression next to
///               an aggregate, and `ORDER BY` over columns and keys.
/// Expectation: The first is rejected naming the implicit keys; the others
///              pass.
#[test]
fn order_by_implicit_grouping_keys_are_rejected() {
    let query = "MATCH (me)--(you) RETURN me.age + you.age, count(*) AS cnt \
                 ORDER BY me.age + you.age + count(*)";
    assert_eq!(
        non_grouping(query, &Catalog::default()),
        vec!["me.age", "you.age"]
    );
    for query in [
        "MATCH (n) RETURN n.k, count(*) AS c ORDER BY n.k + count(*)",
        "MATCH (n) RETURN n.k AS k, count(*) AS c ORDER BY c + count(*), k",
        "MATCH (n) WITH n, count(*) AS c ORDER BY n.x + count(*) RETURN n",
    ] {
        let found = non_grouping(query, &Catalog::default());
        assert!(found.is_empty(), "query: {query}, got: {found:?}");
    }
}
