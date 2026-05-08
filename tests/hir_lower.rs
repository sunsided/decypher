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

// ── Query-time variable substitution ─────────────────────────────────────────

/// `CREATE (p:Person {name: $name}) RETURN p` lowers successfully.
///
/// Unit: `analyze()`
/// Precondition: CREATE uses a parameter as a map property value.
/// Expectation: `analyze` returns `Ok` with one query part.
#[test]
fn analyze_create_with_parameter_value() {
    let hir = analyze("CREATE (p:Person {name: $name}) RETURN p").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

/// `MATCH (n) SET n.name = $name RETURN n` lowers successfully.
///
/// Unit: `analyze()`
/// Precondition: SET clause assigns a query parameter to a static property.
/// Expectation: `analyze` returns `Ok` with one query part containing a Set operation.
#[test]
fn analyze_set_property_parameter_value() {
    use cypher::hir::ops::Operation;
    let hir = analyze("MATCH (n) SET n.name = $name RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let has_set = hir.parts[0]
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Set(_)));
    assert!(has_set, "expected a Set operation");
}

/// `MATCH (n) SET n[$key] = $value RETURN n` lowers to a SetDynamicProperty item.
///
/// Unit: `analyze()`
/// Precondition: SET clause uses a parameter as a dynamic property key.
/// Expectation: `analyze` returns `Ok` and the Set operation contains a
///              `SetDynamicProperty` item.
#[test]
fn analyze_set_dynamic_property_key() {
    use cypher::hir::ops::{Operation, SetItem};
    let hir = analyze("MATCH (n) SET n[$key] = $value RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let set_op = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Set(s) = op {
                Some(s)
            } else {
                None
            }
        })
        .expect("expected a Set operation");
    assert_eq!(set_op.items.len(), 1);
    assert!(
        matches!(set_op.items[0], SetItem::SetDynamicProperty { .. }),
        "expected SetDynamicProperty item, got {:?}",
        set_op.items[0]
    );
}

/// `MERGE … ON CREATE SET p.name = $name ON MATCH SET p.lastSeenAt = datetime()` lowers successfully.
///
/// Unit: `analyze()`
/// Precondition: MERGE with ON CREATE and ON MATCH SET actions using parameters.
/// Expectation: `analyze` returns `Ok` with one query part containing a Merge operation.
#[test]
fn analyze_merge_with_parameter_values() {
    use cypher::hir::ops::Operation;
    let hir = analyze(
        "MERGE (p:Person {externalId: $externalId}) \
         ON CREATE SET p.name = $name \
         ON MATCH SET p.lastSeenAt = datetime() \
         RETURN p",
    )
    .unwrap();
    assert_eq!(hir.parts.len(), 1);
    let has_merge = hir.parts[0]
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Merge(_)));
    assert!(has_merge, "expected a Merge operation");
}
