//! Round-trip tests: verify that parsing → serialising → re-parsing produces
//! identical ASTs.
//!
//! Each test parses a Cypher fragment, serialises it back to a Cypher string
//! via [`ToCypher::to_cypher`], re-parses the result, and checks that the two
//! ASTs are equal.

use assert2::check;
use decypher::ast::ToCypher;
use decypher::parse;

/// Parse `input`, serialise with `to_cypher`, re-parse, and assert AST equality.
fn roundtrip(input: &str) {
    let parsed = parse(input).expect("parse should succeed");
    let emitted = parsed.to_cypher();
    let reparsed = parse(&emitted).expect("reparse should succeed");
    check!(
        parsed == reparsed,
        "round-trip failed.\noriginal:  {}\nemitted:   {}\nparsed ASTs differ",
        input,
        emitted
    );
}

/// Round-trip `rt match return`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_match_return() {
    roundtrip("MATCH (n) RETURN n;");
}

/// Round-trip `rt match where`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_match_where() {
    roundtrip("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
}

/// Round-trip `rt match create`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_match_create() {
    roundtrip("MATCH (a) CREATE (a)-[:KNOWS]->(b);");
}

/// Round-trip `rt match merge`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_match_merge() {
    roundtrip("MATCH (a) MERGE (a)-[:KNOWS]->(b);");
}

/// Round-trip `rt match with return`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_match_with_return() {
    roundtrip("MATCH (n) WITH n.name AS name RETURN name;");
}

/// Round-trip `rt unwind`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_unwind() {
    roundtrip("UNWIND [1, 2, 3] AS x RETURN x;");
}

/// Round-trip `rt set`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_set() {
    roundtrip("MATCH (n) SET n.name = 'Bob';");
}

/// Round-trip `rt set add`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_set_add() {
    roundtrip("MATCH (n) SET n += {extra: 'data'};");
}

/// Round-trip `rt remove`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_remove() {
    roundtrip("MATCH (n) REMOVE n.name;");
}

/// Round-trip `rt delete`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_delete() {
    roundtrip("MATCH (n) DELETE n;");
}

/// Round-trip `rt detach delete`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_detach_delete() {
    roundtrip("MATCH (n) DETACH DELETE n;");
}

/// Round-trip `rt union`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_union() {
    roundtrip("MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;");
}

/// Round-trip `rt union all`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_union_all() {
    roundtrip("MATCH (n:Person) RETURN n.name UNION ALL MATCH (m:Movie) RETURN m.title;");
}

/// Round-trip `rt rich label expression`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_rich_label_expression() {
    roundtrip("MATCH (n:(Person|Company)&!Deleted) RETURN n;");
}

/// Round-trip `rt dynamic node label`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_dynamic_node_label() {
    roundtrip("MATCH (n:$(label)) RETURN n;");
}

/// Round-trip `rt quantified path pattern`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_quantified_path_pattern() {
    roundtrip("MATCH p = ((a:Station)-[:LINK]->(b:Station)){1,3} RETURN p;");
}

/// Round-trip `rt quantified relationship`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_quantified_relationship() {
    roundtrip("MATCH p = (:Station {name: 'A'})-[:LINK]->{1,3}(:Station {name: 'B'}) RETURN p;");
}

/// Round-trip `rt count subquery`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_count_subquery() {
    roundtrip("RETURN COUNT { MATCH (n:Person) RETURN n };");
}

/// Round-trip `rt collect subquery`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_collect_subquery() {
    roundtrip("RETURN COLLECT { MATCH (n:Person) RETURN n.name };");
}

/// Round-trip `rt postfix label expression`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_postfix_label_expression() {
    roundtrip("MATCH (n) WHERE n:Person|Company RETURN n;");
}

/// Round-trip `rt parameters`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_parameters() {
    roundtrip("MATCH (n) WHERE n.name = $name RETURN n;");
}

/// Round-trip `rt comparison chain`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_comparison_chain() {
    roundtrip("MATCH (n) WHERE n.age > 18 AND n.age < 65 RETURN n;");
}

/// Round-trip `rt list literal`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_list_literal() {
    roundtrip("RETURN [1, 2, 3];");
}

/// Round-trip `rt map literal`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_map_literal() {
    roundtrip("RETURN {key: 'value'};");
}

/// Round-trip `rt optional match`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_optional_match() {
    roundtrip("OPTIONAL MATCH (n) RETURN n;");
}

/// Round-trip `rt order skip limit`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_order_skip_limit() {
    roundtrip("MATCH (n) RETURN n ORDER BY n.name ASC SKIP 5 LIMIT 10;");
}

/// Round-trip `rt case expression`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_case_expression() {
    roundtrip("MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END;");
}

/// Round-trip `rt count star`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_count_star() {
    roundtrip("MATCH (n) RETURN COUNT(*);");
}

/// Round-trip `rt in expression`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_in_expression() {
    roundtrip("MATCH (n) WHERE n.name IN ['Alice', 'Bob'] RETURN n;");
}

/// Round-trip `rt is null`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_is_null() {
    roundtrip("MATCH (n) WHERE n.name IS NULL RETURN n;");
}

/// Round-trip `rt is not null`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_is_not_null() {
    roundtrip("MATCH (n) WHERE n.name IS NOT NULL RETURN n;");
}

/// Round-trip `rt starts with`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_starts_with() {
    roundtrip("MATCH (n) WHERE n.name STARTS WITH 'A' RETURN n;");
}

/// Round-trip `rt ends with`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_ends_with() {
    roundtrip("MATCH (n) WHERE n.name ENDS WITH 'z' RETURN n;");
}

/// Round-trip `rt contains`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_contains() {
    roundtrip("MATCH (n) WHERE n.name CONTAINS 'li' RETURN n;");
}

/// Round-trip `rt string literal single quotes`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_string_literal_single_quotes() {
    roundtrip("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
}

/// Round-trip `rt distinct`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_distinct() {
    roundtrip("MATCH (n) RETURN DISTINCT n.name;");
}

/// Round-trip `rt with pipeline`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_with_pipeline() {
    roundtrip("MATCH (n) WITH n.name AS name ORDER BY name RETURN name;");
}

/// Round-trip `rt multi part query`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_multi_part_query() {
    roundtrip("MATCH (a) WITH a MATCH (a)-[:KNOWS]->(b) RETURN b;");
}

/// Round-trip `rt function call`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_function_call() {
    roundtrip("RETURN toUpper('hello');");
}

/// Built-in functions whose names are also keywords are emitted without
/// backticks.
#[test]
fn function_name_keywords_are_not_quoted() {
    let parsed = parse("MATCH (m) RETURN count(m), collect(m), sum(m.x), exists(m.x);")
        .expect("parse should succeed");
    let emitted = parsed.to_cypher();

    assert_eq!(
        emitted,
        "MATCH (m) RETURN count(m), collect(m), sum(m.x), exists(m.x);"
    );
}

/// Every keyword-like name survives printing as a function or procedure name,
/// whichever position it is in: names the parser would read as something else
/// (`not(1)` as a negation, `null(1)` as a literal) must stay quoted, the rest
/// may be printed bare.
///
/// `roundtrip` can't be used here because dropping backticks shifts spans, so
/// this checks that the output re-parses and prints identically instead. A
/// name that changes meaning when bare (`not(1)` → `NOT (1)`) or stops parsing
/// fails that check.
#[test]
fn rt_keyword_function_and_procedure_names() {
    let names = [
        "match",
        "where",
        "return",
        "create",
        "delete",
        "set",
        "remove",
        "with",
        "order",
        "by",
        "asc",
        "desc",
        "skip",
        "limit",
        "union",
        "all",
        "distinct",
        "optional",
        "unwind",
        "merge",
        "on",
        "as",
        "in",
        "is",
        "null",
        "not",
        "and",
        "or",
        "xor",
        "true",
        "false",
        "case",
        "when",
        "then",
        "else",
        "end",
        "exists",
        "count",
        "yield",
        "detach",
        "start",
        "call",
        "filter",
        "reduce",
        "extract",
        "any",
        "none",
        "single",
        "foreach",
        "use",
        "finish",
        "load",
        "show",
        "starts",
        "ends",
        "contains",
        "collect",
        "shortestPath",
        "allShortestPaths",
    ];
    let templates = [
        "RETURN `{}`(1);",
        "RETURN a.`{}`(1);",
        "RETURN `{}`.f(1);",
        "CALL `{}`(1);",
        "CALL a.`{}`(1);",
        "CALL a.`{}`;",
    ];
    for template in templates {
        for name in names {
            let input = template.replace("{}", name);
            let emitted = parse(&input).expect("parse should succeed").to_cypher();
            let reparsed = parse(&emitted)
                .unwrap_or_else(|e| panic!("{input} printed as {emitted}, which fails: {e}"));
            check!(
                reparsed.to_cypher() == emitted,
                "{input} printed as {emitted}, which re-prints as {}",
                reparsed.to_cypher()
            );
        }
    }
}

/// Round-trip `rt function call qualified`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_function_call_qualified() {
    roundtrip("RETURN apoc.text.join(['a', 'b'], '-');");
}

/// Round-trip `rt boolean literals`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_boolean_literals() {
    roundtrip("RETURN true, false;");
}

/// Round-trip `rt null literal`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_null_literal() {
    roundtrip("RETURN NULL;");
}

/// Round-trip `rt arithmetic`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_arithmetic() {
    roundtrip("RETURN 1 + 2 * 3;");
}

/// Round-trip `rt property access`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_property_access() {
    roundtrip("MATCH (n) RETURN n.name.first;");
}

/// Round-trip `rt not operator`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_not_operator() {
    roundtrip("MATCH (n) WHERE NOT n.active RETURN n;");
}

/// Round-trip `rt arithmetic parens`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_arithmetic_parens() {
    roundtrip("RETURN (1 + 2) * 3;");
}

/// Round-trip `rt string with escape`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_string_with_escape() {
    roundtrip("MATCH (n) WHERE n.name = 'O\\'Brien' RETURN n;");
}

/// Round-trip `rt variable length path`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_variable_length_path() {
    roundtrip("MATCH (a)-[:KNOWS*1..5]->(b) RETURN a, b;");
}

/// Round-trip `rt unbounded variable length`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_unbounded_variable_length() {
    roundtrip("MATCH (a)-[:KNOWS*]->(b) RETURN a, b;");
}

/// Round-trip `rt node with labels and props`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_node_with_labels_and_props() {
    roundtrip("MATCH (n:Person {name: 'Alice'}) RETURN n;");
}

/// Round-trip `rt relationship with props`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_relationship_with_props() {
    roundtrip("MATCH (a)-[r:KNOWS {since: 2020}]->(b) RETURN r;");
}

/// Round-trip `rt return order by desc`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_return_order_by_desc() {
    roundtrip("MATCH (n) RETURN n ORDER BY n.age DESC;");
}

/// Round-trip `rt return order by default`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_return_order_by_default() {
    roundtrip("MATCH (n) RETURN n ORDER BY n.name;");
}

/// Round-trip `rt list index`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_list_index() {
    roundtrip("RETURN [1, 2, 3][0];");
}

/// Round-trip `rt list slice`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_list_slice() {
    roundtrip("RETURN [1, 2, 3][1..3];");
}

/// Round-trip `rt list slice open start`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_list_slice_open_start() {
    roundtrip("RETURN [1, 2, 3][..2];");
}

/// Round-trip `rt list slice open end`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_list_slice_open_end() {
    roundtrip("RETURN [1, 2, 3][1..];");
}

/// Round-trip `rt merge on match set`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_merge_on_match_set() {
    roundtrip("MERGE (n:Person {name: 'Alice'}) ON MATCH SET n.lastSeen = timestamp();");
}

/// Round-trip `rt merge on create set`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_merge_on_create_set() {
    roundtrip("MERGE (n:Person {name: 'Alice'}) ON CREATE SET n.created = timestamp();");
}

/// Round-trip `rt float literal`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_float_literal() {
    roundtrip("RETURN 3.14;");
}

/// Round-trip `rt integer literal`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_integer_literal() {
    roundtrip("RETURN 42;");
}

/// Round-trip `rt xor operator`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_xor_operator() {
    roundtrip("MATCH (n) WHERE n.a XOR n.b RETURN n;");
}

/// Round-trip `rt comparison ne`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_comparison_ne() {
    roundtrip("MATCH (n) WHERE n.name <> 'Alice' RETURN n;");
}

/// Round-trip `rt comparison le`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_comparison_le() {
    roundtrip("MATCH (n) WHERE n.age <= 30 RETURN n;");
}

/// Round-trip `rt comparison ge`: parse → serialise → re-parse must yield equal ASTs.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Input Cypher is syntactically valid.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_comparison_ge() {
    roundtrip("MATCH (n) WHERE n.age >= 18 RETURN n;");
}

// ── Query-time variable substitution ─────────────────────────────────────────

/// Round-trip `SET n[$key] = $value`: dynamic property key survives re-serialisation.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: SET clause uses a parameter as a dynamic property key.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_set_dynamic_property_key() {
    roundtrip("MATCH (n) SET n[$key] = $value RETURN n;");
}

/// Round-trip `CREATE (p:Person {name: $name})`: parameter value survives re-serialisation.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: CREATE uses a parameter as a map property value.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_create_with_parameter_value() {
    roundtrip("CREATE (p:Person {name: $name}) RETURN p;");
}

/// Round-trip MERGE with ON CREATE / ON MATCH parameter values.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: MERGE with ON CREATE and ON MATCH SET actions using parameters.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_merge_with_parameter_values() {
    roundtrip(
        "MERGE (p:Person {externalId: $externalId}) \
         ON CREATE SET p.name = $name \
         ON MATCH SET p.lastSeenAt = datetime() \
         RETURN p;",
    );
}

/// Round-trip `SET n.props[$key] = $value`: chained property lookup with dynamic key.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: SET clause chains a static property lookup with a dynamic key.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_set_dynamic_property_key_after_lookup() {
    roundtrip("MATCH (n) SET n.props[$key] = $value RETURN n;");
}

/// Round-trip `SET n[$k1] = $v1, n[$k2] = $v2`: multiple dynamic property assignments.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: SET clause contains two dynamic-key items.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_set_multiple_dynamic_property_keys() {
    roundtrip("MATCH (n) SET n[$k1] = $v1, n[$k2] = $v2 RETURN n;");
}

/// Round-trip `SET n["propName"] = $v`: string literal as the dynamic key.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: SET clause uses a string literal as the property key expression.
/// Expectation: Re-parsed AST equals the original parsed AST.
#[test]
fn rt_set_dynamic_property_literal_key() {
    roundtrip("MATCH (n) SET n[\"propName\"] = $v RETURN n;");
}

/// Round-trip the pattern form of `COUNT { … }` (#107).
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: `COUNT { pattern }` with and without `WHERE`, next to the
///   subquery form.
/// Expectation: Re-parsed AST equals the original parsed AST, and the pattern
///   form prints as a pattern.
#[test]
fn rt_count_pattern_subquery() {
    roundtrip("MATCH (n) WHERE COUNT { (n)-[:R]->() } > 1 RETURN n;");
    roundtrip("MATCH (n) RETURN COUNT { (n)-[:KNOWS]->(m) WHERE m.age > 3 } AS c;");
    roundtrip("MATCH (n) RETURN COUNT { MATCH (n)-[:R]->(m) RETURN m } AS c;");
    let printed = parse("MATCH (n) RETURN COUNT { (n)-[:R]->(m) WHERE m.x = 1 } AS c")
        .unwrap()
        .to_cypher();
    check!(
        printed.contains("COUNT { (n)-[:R]->(m) WHERE m.x = 1 }"),
        "{printed}"
    );
}

/// Round-trip `UNION` chains inside subqueries: every branch survives, with
/// its `ALL` flag, in `CALL { … }` and in `EXISTS` / `COUNT` / `COLLECT`
/// bodies, which may end without `RETURN`.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Subqueries with two and three `UNION` branches.
/// Expectation: Re-parsed AST equals the original parsed AST, the text is
///              unchanged, and the AST holds every branch.
#[test]
fn rt_union_chains_in_subqueries() {
    for (query, unions) in [
        (
            "CALL { RETURN 1 AS a UNION RETURN 2 AS a UNION RETURN 3 AS a } RETURN a;",
            2,
        ),
        (
            "CALL { RETURN 1 AS a UNION ALL RETURN 2 AS a UNION ALL RETURN 3 AS a } RETURN a;",
            2,
        ),
        (
            "MATCH (n) RETURN COUNT { RETURN 1 AS a UNION RETURN 2 AS a UNION RETURN 3 AS a } AS c;",
            2,
        ),
        (
            "MATCH (n) WHERE EXISTS { MATCH (n)-[:R]->(m) UNION MATCH (n)<-[:R]-(m) } RETURN n;",
            1,
        ),
        (
            "RETURN COLLECT { RETURN 1 AS a UNION ALL RETURN 2 AS a } AS c;",
            1,
        ),
    ] {
        roundtrip(query);
        let printed = parse(query).unwrap().to_cypher();
        check!(printed == query);
        check!(printed.matches("UNION").count() == unions);
    }
}

/// Round-trip relationships without a detail block and with an empty one
/// (#113): `-->` and `-[]->` each print as written, in every direction.
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Bare and empty-bracket relationships, including `<-->`,
///               quantified ones and ones inside `EXISTS` / `COUNT`.
/// Expectation: Re-parsed AST equals the original parsed AST, and the text is
///              unchanged.
#[test]
fn rt_bare_and_empty_relationships() {
    for query in [
        "MATCH (a)-->(b)<--(c)--(d)<-->(e) RETURN a;",
        "MATCH (a)-[]->(b)<-[]-(c)-[]-(d)<-[]->(e) RETURN a;",
        "MATCH (a)-->{1,3}(b) RETURN a;",
        "MATCH (n) WHERE EXISTS { (n)-->() } RETURN n;",
        "MATCH (n) RETURN COUNT { (n)<--() } AS c;",
    ] {
        roundtrip(query);
        check!(parse(query).unwrap().to_cypher() == query);
    }
}

/// Round-trip calls of functions whose names are also keywords: `point`
/// (`CREATE POINT INDEX`), `range` and `graph` (`USE graph.byName(…)`).
///
/// Unit: `ToCypher::to_cypher` / `parse`
/// Precondition: Calls of `point`, `point.distance`, `range` and `graph.names`.
/// Expectation: They parse as function calls and print unchanged.
#[test]
fn rt_keyword_named_functions() {
    for query in [
        "RETURN point({x: 1, y: 2}).x AS x;",
        "RETURN point.distance(point({x: 1, y: 2}), point({x: 2, y: 2})) AS d;",
        "RETURN range(1, 3) AS r, graph.names() AS g;",
    ] {
        roundtrip(query);
        check!(parse(query).unwrap().to_cypher() == query);
    }
}
