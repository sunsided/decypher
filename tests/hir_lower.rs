//! Integration tests for the HIR lowering pass (`cypher::analyze`).
//!
//! These tests call [`cypher::analyze`] on Cypher strings and verify the
//! shape of the resulting [`cypher::hir::HirQuery`].

use cypher::analyze;
use cypher::hir::{
    RelationshipDirection,
    ops::{MatchOp, Operation},
};

fn find_match_operation(operations: &[Operation]) -> &MatchOp {
    operations
        .iter()
        .find_map(|op| {
            if let Operation::Match(m) = op {
                Some(m)
            } else {
                None
            }
        })
        .expect("expected a Match operation")
}

/// A basic `MATCH … RETURN` query lowers to exactly one query part.
///
/// Unit: `analyze()`
/// Precondition: A single-part query with a MATCH and a RETURN clause.
/// Expectation: `hir.parts.len() == 1`.
#[test]
fn analyze_basic_query() {
    let hir = analyze("MATCH (p:Person)-[:KNOWS]->(f) WHERE p.age > 18 RETURN f.name").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

/// A `WITH`-split query lowers to two query parts.
///
/// Unit: `analyze()`
/// Precondition: Query has a MATCH → WITH → RETURN structure (two parts).
/// Expectation: `hir.parts.len() == 2`.
#[test]
fn analyze_multi_part() {
    let hir = analyze("MATCH (p:Person) WITH p, count(*) AS cnt WHERE cnt > 3 RETURN p.name, cnt")
        .unwrap();
    assert_eq!(hir.parts.len(), 2);
}

/// Referencing an unbound variable (`x`) produces an error.
///
/// Unit: `analyze()`
/// Precondition: `x` is never bound in the query.
/// Expectation: `analyze` returns `Err`.
#[test]
fn analyze_unknown_variable() {
    let result = analyze("MATCH (p:Person) RETURN x.name");
    assert!(result.is_err());
}

/// A `CREATE` query with no RETURN lowers to one query part.
///
/// Unit: `analyze()`
/// Precondition: Single CREATE clause with inline property map.
/// Expectation: `hir.parts.len() == 1`.
#[test]
fn analyze_create_query() {
    let hir = analyze("CREATE (p:Person {name: 'Alice'})").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

/// An `OPTIONAL MATCH … RETURN` query lowers to one query part.
///
/// Unit: `analyze()`
/// Precondition: OPTIONAL MATCH with a single node pattern and RETURN.
/// Expectation: `hir.parts.len() == 1`.
#[test]
fn analyze_optional_match() {
    let hir = analyze("OPTIONAL MATCH (p:Person) RETURN p.name").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

#[test]
fn analyze_from_preparsed_query() {
    let query =
        cypher::parse("MATCH (p:Person)-[:KNOWS]->(f) WHERE p.age > 18 RETURN f.name").unwrap();
    let hir = analyze(query).unwrap();
    assert_eq!(hir.parts.len(), 1);
}

#[test]
fn analyze_from_preparsed_query_multi_part() {
    let query =
        cypher::parse("MATCH (p:Person) WITH p, count(*) AS cnt WHERE cnt > 3 RETURN p.name, cnt")
            .unwrap();
    let hir = analyze(query).unwrap();
    assert_eq!(hir.parts.len(), 2);
}

#[test]
fn analyze_str_and_query_produce_same_result() {
    let input = "MATCH (p:Person)-[:KNOWS]->(f) RETURN f.name";
    let hir_from_str = analyze(input).unwrap();
    let query = cypher::parse(input).unwrap();
    let hir_from_query = analyze(query).unwrap();
    assert_eq!(hir_from_str.parts.len(), hir_from_query.parts.len());
}

#[test]
fn try_from_str_for_query() {
    use std::convert::TryFrom;
    let query = cypher::Query::try_from("MATCH (n) RETURN n").unwrap();
    assert!(!query.statements.is_empty());
}

#[test]
fn try_from_str_for_query_invalid() {
    use std::convert::TryFrom;
    let result = cypher::Query::try_from("INVALID !!!");
    assert!(result.is_err());
}

#[test]
fn analyze_left_directed_relationship_lowers_to_right_to_left() {
    let hir = analyze("MATCH (a)<-[:T]-(b) RETURN a").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let m = find_match_operation(&hir.parts[0].operations);

    assert_eq!(m.pattern.relationships.len(), 1);
    assert_eq!(
        m.pattern.relationships[0].direction,
        RelationshipDirection::RightToLeft
    );
}

/// Relationships in a chained path must track the correct left (source) node.
///
/// Unit: `lower_pattern_element`
/// Precondition: Two-hop path `(a)-[:E]->(b)-[:F]->(c)`.
/// Expectation: `rel[0].left=0, rel[0].right=1`; `rel[1].left=1, rel[1].right=2`.
#[test]
fn chained_path_relationship_left_indices() {
    let hir = analyze("MATCH (a)-[:E]->(b)-[:F]->(c) RETURN a").unwrap();
    let part = &hir.parts[0];

    let m = find_match_operation(&part.operations);
    let rels = &m.pattern.relationships;
    assert_eq!(rels.len(), 2, "expected two relationships");
    assert_eq!(rels[0].left, 0, "rel[0].left should be 0");
    assert_eq!(rels[0].right, 1, "rel[0].right should be 1");
    assert_eq!(rels[1].left, 1, "rel[1].left should be 1, not 0");
    assert_eq!(rels[1].right, 2, "rel[1].right should be 2");
}
