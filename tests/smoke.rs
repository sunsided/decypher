use assert2::check;
use cypher::parse;

#[test]
fn test_exists_pattern_function() {
    let result = parse("RETURN exists((p)-[:WORKS_AT]->(:Company {name: 'Neo4j'})) AS x");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_match_return() {
    let result = parse("MATCH (n) RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_match_where() {
    let result = parse("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_match_create() {
    let result = parse("MATCH (a) CREATE (a)-[:KNOWS]->(b);");
    check!(result.is_ok());
}

#[test]
fn test_match_merge() {
    let result = parse("MATCH (a) MERGE (a)-[:KNOWS]->(b);");
    check!(result.is_ok());
}

#[test]
fn test_match_with_return() {
    let result = parse("MATCH (n) WITH n.name AS name RETURN name;");
    check!(result.is_ok());
}

#[test]
fn test_unwind() {
    let result = parse("UNWIND [1, 2, 3] AS x RETURN x;");
    check!(result.is_ok());
}

#[test]
fn test_set() {
    let result = parse("MATCH (n) SET n.name = 'Bob';");
    check!(result.is_ok());
}

#[test]
fn test_remove() {
    let result = parse("MATCH (n) REMOVE n.name;");
    check!(result.is_ok());
}

#[test]
fn test_detach_delete() {
    let result = parse("MATCH (n) DETACH DELETE n;");
    check!(result.is_ok());
}

#[test]
fn test_union() {
    let result = parse("MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;");
    check!(result.is_ok());
}

#[test]
fn test_union_all() {
    let result = parse("MATCH (n:Person) RETURN n.name UNION ALL MATCH (m:Movie) RETURN m.title;");
    check!(result.is_ok());
}

#[test]
fn test_parameters() {
    let result = parse("MATCH (n) WHERE n.name = $name RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_comments() {
    let result = parse("MATCH (n) // comment\n RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_comparison_chain() {
    let result = parse("MATCH (n) WHERE n.age > 18 AND n.age < 65 RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_list_literal() {
    let result = parse("RETURN [1, 2, 3];");
    check!(result.is_ok());
}

#[test]
fn test_map_literal() {
    let result = parse("RETURN {key: 'value'};");
    check!(result.is_ok());
}

#[test]
fn test_optional_match() {
    let result = parse("OPTIONAL MATCH (n) RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_order_skip_limit() {
    let result = parse("MATCH (n) RETURN n ORDER BY n.name ASC SKIP 5 LIMIT 10;");
    check!(result.is_ok());
}

#[test]
fn test_case_expression() {
    let result = parse("MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END;");
    check!(result.is_ok());
}

#[test]
fn test_count_star() {
    let result = parse("MATCH (n) RETURN COUNT(*);");
    check!(result.is_ok());
}

#[test]
fn test_in_expression() {
    let result = parse("MATCH (n) WHERE n.name IN ['Alice', 'Bob'] RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_is_null() {
    let result = parse("MATCH (n) WHERE n.name IS NULL RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_is_not_null() {
    let result = parse("MATCH (n) WHERE n.name IS NOT NULL RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_starts_with() {
    let result = parse("MATCH (n) WHERE n.name STARTS WITH 'A' RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_string_literal_single_quotes() {
    let result = parse("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_string_literal_double_quotes() {
    let result = parse("MATCH (n) WHERE n.name = \"Alice\" RETURN n;");
    check!(result.is_ok());
}

#[test]
fn test_distinct() {
    let result = parse("MATCH (n) RETURN DISTINCT n.name;");
    check!(result.is_ok());
}

#[test]
fn test_with_pipeline() {
    let result = parse("MATCH (n) WITH n.name AS name ORDER BY name RETURN name;");
    check!(result.is_ok());
}

#[test]
fn test_multi_part_query() {
    let result = parse("MATCH (a) WITH a MATCH (a)-[:KNOWS]->(b) RETURN b;");
    check!(result.is_ok());
}

#[test]
fn test_function_call() {
    let result = parse("RETURN toUpper('hello');");
    check!(result.is_ok());
}

#[test]
fn test_boolean_literals() {
    let result = parse("RETURN true, false;");
    check!(result.is_ok());
}

#[test]
fn test_null_literal() {
    let result = parse("RETURN null;");
    check!(result.is_ok());
}

#[test]
fn test_arithmetic() {
    let result = parse("RETURN 1 + 2 * 3;");
    check!(result.is_ok());
}

#[test]
fn test_property_access() {
    let result = parse("MATCH (n) RETURN n.name.first;");
    check!(result.is_ok());
}

#[test]
fn test_composite_node_key() {
    let result = parse(
        "CREATE CONSTRAINT composite_key FOR (p:Person) REQUIRE (p.country, p.id) IS NODE KEY;",
    );
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_index_label_alternatives() {
    let result =
        parse("CREATE FULLTEXT INDEX person_names FOR (p:Person|Employee) ON EACH [p.name];");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_collect_subquery() {
    let result = parse("RETURN COLLECT { MATCH (n) RETURN n };");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_count_subquery() {
    let result = parse("RETURN COUNT { MATCH (n:Person) RETURN n };");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_rich_label_expression() {
    let result = parse("MATCH (n:(Person|Company)&!Deleted) RETURN n;");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_dynamic_node_label() {
    let result = parse("MATCH (n:$(label)) RETURN n;");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_quantified_path_pattern() {
    let result = parse("MATCH p = ((a:Stop WHERE a.active)-[:NEXT]->(b:Stop WHERE b.active)){2,5} RETURN p;");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_quantified_relationship() {
    let result = parse("MATCH p = (:Person)-[r:KNOWS WHERE r.weight > 0.5]->{1,4}(:Person) RETURN p;");
    check!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_postfix_label_expression() {
    let result = parse("MATCH (n) WHERE n:Person|Company RETURN n;");
    check!(result.is_ok(), "{:?}", result.err());
}
