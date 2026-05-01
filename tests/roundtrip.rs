use open_cypher::ast::ToCypher;
use open_cypher::parse;

fn roundtrip(input: &str) {
    let parsed = parse(input).expect("parse should succeed");
    let emitted = parsed.to_cypher();
    let reparsed = parse(&emitted).expect("reparse should succeed");
    assert_eq!(
        parsed, reparsed,
        "round-trip failed.\noriginal:  {}\nemitted:   {}\nparsed ASTs differ",
        input, emitted
    );
}

#[test]
fn rt_match_return() {
    roundtrip("MATCH (n) RETURN n;");
}

#[test]
fn rt_match_where() {
    roundtrip("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
}

#[test]
fn rt_match_create() {
    roundtrip("MATCH (a) CREATE (a)-[:KNOWS]->(b);");
}

#[test]
fn rt_match_merge() {
    roundtrip("MATCH (a) MERGE (a)-[:KNOWS]->(b);");
}

#[test]
fn rt_match_with_return() {
    roundtrip("MATCH (n) WITH n.name AS name RETURN name;");
}

#[test]
fn rt_unwind() {
    roundtrip("UNWIND [1, 2, 3] AS x RETURN x;");
}

#[test]
fn rt_set() {
    roundtrip("MATCH (n) SET n.name = 'Bob';");
}

#[test]
fn rt_set_add() {
    roundtrip("MATCH (n) SET n += {extra: 'data'};");
}

#[test]
fn rt_remove() {
    roundtrip("MATCH (n) REMOVE n.name;");
}

#[test]
fn rt_delete() {
    roundtrip("MATCH (n) DELETE n;");
}

#[test]
fn rt_detach_delete() {
    roundtrip("MATCH (n) DETACH DELETE n;");
}

#[test]
fn rt_union() {
    roundtrip("MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;");
}

#[test]
fn rt_union_all() {
    roundtrip("MATCH (n:Person) RETURN n.name UNION ALL MATCH (m:Movie) RETURN m.title;");
}

#[test]
fn rt_parameters() {
    roundtrip("MATCH (n) WHERE n.name = $name RETURN n;");
}

#[test]
fn rt_comparison_chain() {
    roundtrip("MATCH (n) WHERE n.age > 18 AND n.age < 65 RETURN n;");
}

#[test]
fn rt_list_literal() {
    roundtrip("RETURN [1, 2, 3];");
}

#[test]
fn rt_map_literal() {
    roundtrip("RETURN {key: 'value'};");
}

#[test]
fn rt_optional_match() {
    roundtrip("OPTIONAL MATCH (n) RETURN n;");
}

#[test]
fn rt_order_skip_limit() {
    roundtrip("MATCH (n) RETURN n ORDER BY n.name ASC SKIP 5 LIMIT 10;");
}

#[test]
fn rt_case_expression() {
    roundtrip("MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END;");
}

#[test]
fn rt_count_star() {
    roundtrip("MATCH (n) RETURN COUNT(*);");
}

#[test]
fn rt_in_expression() {
    roundtrip("MATCH (n) WHERE n.name IN ['Alice', 'Bob'] RETURN n;");
}

#[test]
fn rt_is_null() {
    roundtrip("MATCH (n) WHERE n.name IS NULL RETURN n;");
}

#[test]
fn rt_is_not_null() {
    roundtrip("MATCH (n) WHERE n.name IS NOT NULL RETURN n;");
}

#[test]
fn rt_starts_with() {
    roundtrip("MATCH (n) WHERE n.name STARTS WITH 'A' RETURN n;");
}

#[test]
fn rt_ends_with() {
    roundtrip("MATCH (n) WHERE n.name ENDS WITH 'z' RETURN n;");
}

#[test]
fn rt_contains() {
    roundtrip("MATCH (n) WHERE n.name CONTAINS 'li' RETURN n;");
}

#[test]
fn rt_string_literal_single_quotes() {
    roundtrip("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
}

#[test]
fn rt_distinct() {
    roundtrip("MATCH (n) RETURN DISTINCT n.name;");
}

#[test]
fn rt_with_pipeline() {
    roundtrip("MATCH (n) WITH n.name AS name ORDER BY name RETURN name;");
}

#[test]
fn rt_multi_part_query() {
    roundtrip("MATCH (a) WITH a MATCH (a)-[:KNOWS]->(b) RETURN b;");
}

#[test]
fn rt_function_call() {
    roundtrip("RETURN toUpper('hello');");
}

#[test]
fn rt_function_call_qualified() {
    roundtrip("RETURN apoc.text.join(['a', 'b'], '-');");
}

#[test]
fn rt_boolean_literals() {
    roundtrip("RETURN true, false;");
}

#[test]
fn rt_null_literal() {
    roundtrip("RETURN NULL;");
}

#[test]
fn rt_arithmetic() {
    roundtrip("RETURN 1 + 2 * 3;");
}

#[test]
fn rt_property_access() {
    roundtrip("MATCH (n) RETURN n.name.first;");
}

#[test]
fn rt_not_operator() {
    roundtrip("MATCH (n) WHERE NOT n.active RETURN n;");
}

#[test]
fn rt_arithmetic_parens() {
    roundtrip("RETURN (1 + 2) * 3;");
}

#[test]
fn rt_string_with_escape() {
    roundtrip("MATCH (n) WHERE n.name = 'O\\'Brien' RETURN n;");
}

#[test]
fn rt_variable_length_path() {
    roundtrip("MATCH (a)-[:KNOWS*1..5]->(b) RETURN a, b;");
}

#[test]
fn rt_unbounded_variable_length() {
    roundtrip("MATCH (a)-[:KNOWS*]->(b) RETURN a, b;");
}

#[test]
fn rt_node_with_labels_and_props() {
    roundtrip("MATCH (n:Person {name: 'Alice'}) RETURN n;");
}

#[test]
fn rt_relationship_with_props() {
    roundtrip("MATCH (a)-[r:KNOWS {since: 2020}]->(b) RETURN r;");
}

#[test]
fn rt_return_order_by_desc() {
    roundtrip("MATCH (n) RETURN n ORDER BY n.age DESC;");
}

#[test]
fn rt_return_order_by_default() {
    roundtrip("MATCH (n) RETURN n ORDER BY n.name;");
}

#[test]
fn rt_list_index() {
    roundtrip("RETURN [1, 2, 3][0];");
}

#[test]
fn rt_list_slice() {
    roundtrip("RETURN [1, 2, 3][1..3];");
}

#[test]
fn rt_list_slice_open_start() {
    roundtrip("RETURN [1, 2, 3][..2];");
}

#[test]
fn rt_list_slice_open_end() {
    roundtrip("RETURN [1, 2, 3][1..];");
}

#[test]
fn rt_merge_on_match_set() {
    roundtrip("MERGE (n:Person {name: 'Alice'}) ON MATCH SET n.lastSeen = timestamp();");
}

#[test]
fn rt_merge_on_create_set() {
    roundtrip("MERGE (n:Person {name: 'Alice'}) ON CREATE SET n.created = timestamp();");
}

#[test]
fn rt_float_literal() {
    roundtrip("RETURN 3.14;");
}

#[test]
fn rt_integer_literal() {
    roundtrip("RETURN 42;");
}

#[test]
fn rt_xor_operator() {
    roundtrip("MATCH (n) WHERE n.a XOR n.b RETURN n;");
}

#[test]
fn rt_comparison_ne() {
    roundtrip("MATCH (n) WHERE n.name <> 'Alice' RETURN n;");
}

#[test]
fn rt_comparison_le() {
    roundtrip("MATCH (n) WHERE n.age <= 30 RETURN n;");
}

#[test]
fn rt_comparison_ge() {
    roundtrip("MATCH (n) WHERE n.age >= 18 RETURN n;");
}
