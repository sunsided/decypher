//! Smoke tests: verify that common Cypher statement forms parse without error.
//!
//! Each test in this module exercises a small, self-contained Cypher fragment
//! to ensure that the parser accepts it and produces an `Ok` result.

use assert2::check;
use cypher::parse;

/// Parse `exists()` used as a function with a pattern argument.
///
/// Unit: `parse()`
/// Precondition: Cypher source contains `exists((p)-[:WORKS_AT]->(:Company {name: 'Neo4j'}))`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_exists_pattern_function() {
    let result = parse("RETURN exists((p)-[:WORKS_AT]->(:Company {name: 'Neo4j'})) AS x");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a simple `MATCH (n) RETURN n`.
///
/// Unit: `parse()`
/// Precondition: Cypher source is a minimal MATCH/RETURN pair.
/// Expectation: parser returns `Ok`.
#[test]
fn test_match_return() {
    let result = parse("MATCH (n) RETURN n;");
    check!(result.is_ok());
}

/// Parse a `MATCH … WHERE … RETURN …` form.
///
/// Unit: `parse()`
/// Precondition: WHERE clause with a string equality predicate.
/// Expectation: parser returns `Ok`.
#[test]
fn test_match_where() {
    let result = parse("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
    check!(result.is_ok());
}

/// Parse a combined `MATCH … CREATE …` statement.
///
/// Unit: `parse()`
/// Precondition: MATCH followed by CREATE of a new relationship.
/// Expectation: parser returns `Ok`.
#[test]
fn test_match_create() {
    let result = parse("MATCH (a) CREATE (a)-[:KNOWS]->(b);");
    check!(result.is_ok());
}

/// Parse a combined `MATCH … MERGE …` statement.
///
/// Unit: `parse()`
/// Precondition: MATCH followed by MERGE of a relationship.
/// Expectation: parser returns `Ok`.
#[test]
fn test_match_merge() {
    let result = parse("MATCH (a) MERGE (a)-[:KNOWS]->(b);");
    check!(result.is_ok());
}

/// Parse a `MATCH … WITH … RETURN …` multi-part query.
///
/// Unit: `parse()`
/// Precondition: WITH renames a property with AS.
/// Expectation: parser returns `Ok`.
#[test]
fn test_match_with_return() {
    let result = parse("MATCH (n) WITH n.name AS name RETURN name;");
    check!(result.is_ok());
}

/// Parse an `UNWIND … RETURN …` query.
///
/// Unit: `parse()`
/// Precondition: UNWIND over a list literal.
/// Expectation: parser returns `Ok`.
#[test]
fn test_unwind() {
    let result = parse("UNWIND [1, 2, 3] AS x RETURN x;");
    check!(result.is_ok());
}

/// Parse a `MATCH … SET …` mutation statement.
///
/// Unit: `parse()`
/// Precondition: SET clause assigns a string property.
/// Expectation: parser returns `Ok`.
#[test]
fn test_set() {
    let result = parse("MATCH (n) SET n.name = 'Bob';");
    check!(result.is_ok());
}

/// Parse a `MATCH … REMOVE …` mutation statement.
///
/// Unit: `parse()`
/// Precondition: REMOVE clause targets a single property.
/// Expectation: parser returns `Ok`.
#[test]
fn test_remove() {
    let result = parse("MATCH (n) REMOVE n.name;");
    check!(result.is_ok());
}

/// Parse a `MATCH … DETACH DELETE …` mutation statement.
///
/// Unit: `parse()`
/// Precondition: DETACH DELETE clause targets a node variable.
/// Expectation: parser returns `Ok`.
#[test]
fn test_detach_delete() {
    let result = parse("MATCH (n) DETACH DELETE n;");
    check!(result.is_ok());
}

/// Parse a `UNION` query combining two MATCH/RETURN branches.
///
/// Unit: `parse()`
/// Precondition: Two separate MATCH/RETURN clauses joined by `UNION`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_union() {
    let result = parse("MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;");
    check!(result.is_ok());
}

/// Parse a `UNION ALL` query that preserves duplicates.
///
/// Unit: `parse()`
/// Precondition: Two MATCH/RETURN clauses joined by `UNION ALL`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_union_all() {
    let result = parse("MATCH (n:Person) RETURN n.name UNION ALL MATCH (m:Movie) RETURN m.title;");
    check!(result.is_ok());
}

/// Parse a query using a query parameter (`$name`).
///
/// Unit: `parse()`
/// Precondition: WHERE clause references a named parameter `$name`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_parameters() {
    let result = parse("MATCH (n) WHERE n.name = $name RETURN n;");
    check!(result.is_ok());
}

/// Parse a query that contains a single-line comment.
///
/// Unit: `parse()`
/// Precondition: Source contains `// comment` on a separate line.
/// Expectation: parser returns `Ok` and comments are treated as trivia.
#[test]
fn test_comments() {
    let result = parse("MATCH (n) // comment\n RETURN n;");
    check!(result.is_ok());
}

/// Parse a chained comparison with `AND`.
///
/// Unit: `parse()`
/// Precondition: WHERE clause with `n.age > 18 AND n.age < 65`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_comparison_chain() {
    let result = parse("MATCH (n) WHERE n.age > 18 AND n.age < 65 RETURN n;");
    check!(result.is_ok());
}

/// Parse a list literal expression.
///
/// Unit: `parse()`
/// Precondition: `RETURN [1, 2, 3]`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_list_literal() {
    let result = parse("RETURN [1, 2, 3];");
    check!(result.is_ok());
}

/// Parse a map literal expression.
///
/// Unit: `parse()`
/// Precondition: `RETURN {key: 'value'}`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_map_literal() {
    let result = parse("RETURN {key: 'value'};");
    check!(result.is_ok());
}

/// Parse an `OPTIONAL MATCH` clause.
///
/// Unit: `parse()`
/// Precondition: Query uses `OPTIONAL MATCH`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_optional_match() {
    let result = parse("OPTIONAL MATCH (n) RETURN n;");
    check!(result.is_ok());
}

/// Parse `ORDER BY … ASC SKIP … LIMIT …` modifiers on a RETURN clause.
///
/// Unit: `parse()`
/// Precondition: RETURN with ORDER BY, SKIP, and LIMIT.
/// Expectation: parser returns `Ok`.
#[test]
fn test_order_skip_limit() {
    let result = parse("MATCH (n) RETURN n ORDER BY n.name ASC SKIP 5 LIMIT 10;");
    check!(result.is_ok());
}

/// Parse a searched `CASE WHEN … THEN … ELSE … END` expression.
///
/// Unit: `parse()`
/// Precondition: CASE with a single WHEN/THEN branch and an ELSE.
/// Expectation: parser returns `Ok`.
#[test]
fn test_case_expression() {
    let result = parse("MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END;");
    check!(result.is_ok());
}

/// Parse `COUNT(*)` aggregate.
///
/// Unit: `parse()`
/// Precondition: `RETURN COUNT(*)`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_count_star() {
    let result = parse("MATCH (n) RETURN COUNT(*);");
    check!(result.is_ok());
}

/// Parse an `IN [list]` expression.
///
/// Unit: `parse()`
/// Precondition: WHERE clause with an `IN` membership test.
/// Expectation: parser returns `Ok`.
#[test]
fn test_in_expression() {
    let result = parse("MATCH (n) WHERE n.name IN ['Alice', 'Bob'] RETURN n;");
    check!(result.is_ok());
}

/// Parse an `IS NULL` predicate.
///
/// Unit: `parse()`
/// Precondition: WHERE clause with `IS NULL`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_is_null() {
    let result = parse("MATCH (n) WHERE n.name IS NULL RETURN n;");
    check!(result.is_ok());
}

/// Parse an `IS NOT NULL` predicate.
///
/// Unit: `parse()`
/// Precondition: WHERE clause with `IS NOT NULL`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_is_not_null() {
    let result = parse("MATCH (n) WHERE n.name IS NOT NULL RETURN n;");
    check!(result.is_ok());
}

/// Parse a `STARTS WITH` string operator.
///
/// Unit: `parse()`
/// Precondition: WHERE clause with `STARTS WITH 'A'`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_starts_with() {
    let result = parse("MATCH (n) WHERE n.name STARTS WITH 'A' RETURN n;");
    check!(result.is_ok());
}

/// Parse a single-quoted string literal.
///
/// Unit: `parse()`
/// Precondition: String value wrapped in single quotes.
/// Expectation: parser returns `Ok`.
#[test]
fn test_string_literal_single_quotes() {
    let result = parse("MATCH (n) WHERE n.name = 'Alice' RETURN n;");
    check!(result.is_ok());
}

/// Parse a double-quoted string literal.
///
/// Unit: `parse()`
/// Precondition: String value wrapped in double quotes.
/// Expectation: parser returns `Ok`.
#[test]
fn test_string_literal_double_quotes() {
    let result = parse("MATCH (n) WHERE n.name = \"Alice\" RETURN n;");
    check!(result.is_ok());
}

/// Parse `RETURN DISTINCT …`.
///
/// Unit: `parse()`
/// Precondition: RETURN clause with DISTINCT modifier.
/// Expectation: parser returns `Ok`.
#[test]
fn test_distinct() {
    let result = parse("MATCH (n) RETURN DISTINCT n.name;");
    check!(result.is_ok());
}

/// Parse a `WITH … ORDER BY …` pipeline.
///
/// Unit: `parse()`
/// Precondition: WITH clause followed by ORDER BY on the alias.
/// Expectation: parser returns `Ok`.
#[test]
fn test_with_pipeline() {
    let result = parse("MATCH (n) WITH n.name AS name ORDER BY name RETURN name;");
    check!(result.is_ok());
}

/// Parse a multi-part query with two MATCH parts joined by WITH.
///
/// Unit: `parse()`
/// Precondition: First MATCH feeds the second via WITH.
/// Expectation: parser returns `Ok`.
#[test]
fn test_multi_part_query() {
    let result = parse("MATCH (a) WITH a MATCH (a)-[:KNOWS]->(b) RETURN b;");
    check!(result.is_ok());
}

/// Parse a function call expression `toUpper('hello')`.
///
/// Unit: `parse()`
/// Precondition: RETURN a single function call with a string argument.
/// Expectation: parser returns `Ok`.
#[test]
fn test_function_call() {
    let result = parse("RETURN toUpper('hello');");
    check!(result.is_ok());
}

/// Parse `true` and `false` boolean literals.
///
/// Unit: `parse()`
/// Precondition: RETURN with both boolean literals.
/// Expectation: parser returns `Ok`.
#[test]
fn test_boolean_literals() {
    let result = parse("RETURN true, false;");
    check!(result.is_ok());
}

/// Parse the `null` literal.
///
/// Unit: `parse()`
/// Precondition: `RETURN null`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_null_literal() {
    let result = parse("RETURN null;");
    check!(result.is_ok());
}

/// Parse an arithmetic expression with operator precedence.
///
/// Unit: `parse()`
/// Precondition: `RETURN 1 + 2 * 3` (multiplication binds tighter).
/// Expectation: parser returns `Ok`.
#[test]
fn test_arithmetic() {
    let result = parse("RETURN 1 + 2 * 3;");
    check!(result.is_ok());
}

/// Parse a chained property access `n.name.first`.
///
/// Unit: `parse()`
/// Precondition: Property lookup has two levels of nesting.
/// Expectation: parser returns `Ok`.
#[test]
fn test_property_access() {
    let result = parse("MATCH (n) RETURN n.name.first;");
    check!(result.is_ok());
}

/// Parse a `CREATE CONSTRAINT … IS NODE KEY` DDL command.
///
/// Unit: `parse()`
/// Precondition: Composite node key constraint with two properties.
/// Expectation: parser returns `Ok`.
#[test]
fn test_composite_node_key() {
    let result = parse(
        "CREATE CONSTRAINT composite_key FOR (p:Person) REQUIRE (p.country, p.id) IS NODE KEY;",
    );
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a `CREATE FULLTEXT INDEX` with label alternatives.
///
/// Unit: `parse()`
/// Precondition: Full-text index on `Person|Employee` using `ON EACH`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_index_label_alternatives() {
    let result =
        parse("CREATE FULLTEXT INDEX person_names FOR (p:Person|Employee) ON EACH [p.name];");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a `COLLECT { … }` subquery expression.
///
/// Unit: `parse()`
/// Precondition: RETURN wraps a COLLECT subquery.
/// Expectation: parser returns `Ok`.
#[test]
fn test_collect_subquery() {
    let result = parse("RETURN COLLECT { MATCH (n) RETURN n };");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a `COUNT { … }` subquery expression.
///
/// Unit: `parse()`
/// Precondition: RETURN wraps a COUNT subquery filtering by label.
/// Expectation: parser returns `Ok`.
#[test]
fn test_count_subquery() {
    let result = parse("RETURN COUNT { MATCH (n:Person) RETURN n };");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a compound label expression `(Person|Company)&!Deleted`.
///
/// Unit: `parse()`
/// Precondition: Node pattern uses AND, OR, and NOT label operators.
/// Expectation: parser returns `Ok`.
#[test]
fn test_rich_label_expression() {
    let result = parse("MATCH (n:(Person|Company)&!Deleted) RETURN n;");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a dynamic label expression `$(label)`.
///
/// Unit: `parse()`
/// Precondition: Node pattern uses a dynamic label reference.
/// Expectation: parser returns `Ok`.
#[test]
fn test_dynamic_node_label() {
    let result = parse("MATCH (n:$(label)) RETURN n;");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a quantified path pattern `{2,5}` over a two-hop pattern.
///
/// Unit: `parse()`
/// Precondition: Pattern element quantified with a range `{2,5}`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_quantified_path_pattern() {
    let result = parse(
        "MATCH p = ((a:Stop WHERE a.active)-[:NEXT]->(b:Stop WHERE b.active)){2,5} RETURN p;",
    );
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a quantified relationship `->{}` with a range specifier.
///
/// Unit: `parse()`
/// Precondition: Relationship pattern uses `{1,4}` quantifier with a WHERE filter.
/// Expectation: parser returns `Ok`.
#[test]
fn test_quantified_relationship() {
    let result =
        parse("MATCH p = (:Person)-[r:KNOWS WHERE r.weight > 0.5]->{1,4}(:Person) RETURN p;");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Parse a postfix label expression in a WHERE clause `n:Person|Company`.
///
/// Unit: `parse()`
/// Precondition: WHERE clause uses postfix label-test syntax.
/// Expectation: parser returns `Ok`.
#[test]
fn test_postfix_label_expression() {
    let result = parse("MATCH (n) WHERE n:Person|Company RETURN n;");
    check!(result.is_ok(), "{:?}", result.err());
}
