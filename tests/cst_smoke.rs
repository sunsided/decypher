//! CST smoke tests: verify that the rowan-based CST path parses
//! common Cypher queries correctly.

use assert2::check;
use decypher::parse;

/// All smoke-test queries, verified against the rowan parser.
#[test]
fn cst_parity_smoke_queries() {
    let queries: &[&str] = &[
        "MATCH (n) RETURN n;",
        "MATCH (n) WHERE n.name = 'Alice' RETURN n;",
        "MATCH (a) CREATE (a)-[:KNOWS]->(b);",
        "MATCH (a) MERGE (a)-[:KNOWS]->(b);",
        "MATCH (n) WITH n.name AS name RETURN name;",
        "UNWIND [1, 2, 3] AS x RETURN x;",
        "MATCH (n) SET n.name = 'Bob';",
        "MATCH (n) REMOVE n.name;",
        "MATCH (n) DETACH DELETE n;",
        "MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;",
        "MATCH (n:Person) RETURN n.name UNION ALL MATCH (m:Movie) RETURN m.title;",
        "MATCH (n) // comment\n RETURN n;",
        "MATCH (n) WHERE n.age > 18 AND n.age < 65 RETURN n;",
        "RETURN [1, 2, 3];",
        "RETURN {key: 'value'};",
        "OPTIONAL MATCH (n) RETURN n;",
        "MATCH (n) RETURN n ORDER BY n.name ASC SKIP 5 LIMIT 10;",
        "MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END;",
        "MATCH (n) RETURN COUNT(*);",
        "MATCH (n) WHERE n.name IN ['Alice', 'Bob'] RETURN n;",
        "MATCH (n) WHERE n.name IS NULL RETURN n;",
        "MATCH (n) WHERE n.name IS NOT NULL RETURN n;",
        "MATCH (n) WHERE n.name STARTS WITH 'A' RETURN n;",
        "MATCH (n) WHERE n.name = \"Alice\" RETURN n;",
        "MATCH (n) RETURN DISTINCT n.name;",
        "MATCH (a) WITH a MATCH (a)-[:KNOWS]->(b) RETURN b;",
        "RETURN toUpper('hello');",
        "RETURN true, false;",
        "RETURN null;",
        "RETURN 1 + 2 * 3;",
        "MATCH (n) RETURN n.name.first;",
    ];

    for &input in queries {
        let result = parse(input);
        check!(
            result.is_ok(),
            "rowan parse failed for: {input}\nError: {result:?}",
        );
    }
}

/// Expression-level: simple RETURN queries.
#[test]
fn cst_parity_expressions() {
    let queries: &[&str] = &[
        "RETURN 1 + 2 * 3",
        "RETURN NOT true",
        "RETURN name STARTS WITH 'A'",
        "RETURN x IS NOT NULL",
        "RETURN (1 + 2) * 3",
        "RETURN a.b.c.d",
        "RETURN count(n)",
        "RETURN [1, 2, 3]",
        "RETURN {key: 'value'}",
    ];

    for &input in queries {
        let result = parse(input);
        check!(result.is_ok(), "rowan parse failed for: {input}");
    }
}

/// Pattern-level: MATCH queries with various pattern shapes.
#[test]
fn cst_parity_patterns() {
    let queries: &[&str] = &[
        "MATCH (n) RETURN n",
        "MATCH (n:Person) RETURN n.name",
        "MATCH (a)-[r]->(b) RETURN a, r, b",
        "MATCH (n) WHERE n.age > 18 RETURN n",
    ];

    for &input in queries {
        let result = parse(input);
        check!(result.is_ok(), "rowan parse failed for: {input}");
    }
}
