//! The `serde` feature serialises the typed AST (used by the wasm bindings).

use serde_json::json;

#[test]
fn ast_serialises_to_json_with_spans() {
    let query = decypher::parse("MATCH (n:Person) RETURN n").unwrap();
    let value = serde_json::to_value(&query).unwrap();

    assert_eq!(value["span"], json!({ "start": 0, "end": 25 }));
    let single = &value["statements"][0]["SingleQuery"]["kind"]["SinglePart"];
    let pattern = &single["reading_clauses"][0]["Match"]["pattern"]["parts"][0];
    let node = &pattern["anonymous"]["element"]["Path"]["start"];
    assert_eq!(node["variable"]["name"]["name"], "n");
    assert_eq!(node["labels"][0]["Static"]["name"], "Person");
}

#[test]
fn every_suite_query_serialises() {
    // Suite 5 holds queries that must fail to parse.
    for suite in 1..=4 {
        let path = format!("{}/tests/suite-{suite}.cypher", env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(&path).unwrap();
        let query = decypher::parse(source.as_str()).unwrap_or_else(|e| panic!("{path}: {e}"));
        serde_json::to_string(&query).unwrap_or_else(|e| panic!("{path}: {e}"));
    }
}
