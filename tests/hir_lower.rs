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
