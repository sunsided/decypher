use cypher::analyze;

#[test]
fn analyze_basic_query() {
    let hir = analyze("MATCH (p:Person)-[:KNOWS]->(f) WHERE p.age > 18 RETURN f.name").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

#[test]
fn analyze_multi_part() {
    let hir = analyze("MATCH (p:Person) WITH p, count(*) AS cnt WHERE cnt > 3 RETURN p.name, cnt")
        .unwrap();
    assert_eq!(hir.parts.len(), 2);
}

#[test]
fn analyze_unknown_variable() {
    let result = analyze("MATCH (p:Person) RETURN x.name");
    assert!(result.is_err());
}

#[test]
fn analyze_create_query() {
    let hir = analyze("CREATE (p:Person {name: 'Alice'})").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

#[test]
fn analyze_optional_match() {
    let hir = analyze("OPTIONAL MATCH (p:Person) RETURN p.name").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

#[test]
fn analyze_from_preparsed_query() {
    let query = cypher::parse("MATCH (p:Person)-[:KNOWS]->(f) WHERE p.age > 18 RETURN f.name")
        .unwrap();
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
