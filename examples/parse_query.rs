use open_cypher::parse;

fn main() {
    let queries = [
        "MATCH (n) WHERE n.name = 'Alice' RETURN n;",
        "RETURN 1 + 2 * 3;",
        "MATCH (n) WHERE n.age > 18 AND n.age < 65 RETURN n;",
        "MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END;",
        "MATCH (n) RETURN COUNT(*);",
        "MATCH (n) RETURN n ORDER BY n.name ASC SKIP 5 LIMIT 10;",
        "MATCH (n) WHERE n.name = $name RETURN n;",
        "MATCH (n) WHERE n.name = \"Alice\" RETURN n;",
    ];
    for q in &queries {
        match parse(q) {
            Ok(_) => println!("OK: {}", q),
            Err(e) => println!("FAIL: {} -> {}", q, e),
        }
    }
}
