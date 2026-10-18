//! Integration tests that verify the structural shape of the parsed AST.
//!
//! These tests exercise the typed AST node hierarchy by inspecting fields
//! such as `statements`, `reading_clauses`, `body`, etc. to confirm that
//! the parser constructs the correct AST variants for various query shapes.

use assert2::check;
use decypher::ast::query::{QueryBody, SinglePartBody};
use decypher::parse;

/// A `MATCH … RETURN n` query produces a `SinglePartBody::Return` with one
/// projection item.
///
/// Unit: `parse()` / AST `SinglePartBody`
/// Precondition: `MATCH (n) RETURN n;` — single MATCH and a RETURN with one item.
/// Expectation: AST has `SinglePartBody::Return` with `items.len() == 1`.
#[test]
fn test_match_return_has_return_body() {
    let query = parse("MATCH (n) RETURN n;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            decypher::ast::query::SingleQueryKind::SinglePart(spq) => match &spq.body {
                SinglePartBody::Return(ret) => {
                    check!(ret.items.len() == 1);
                }
                _ => panic!("expected Return body"),
            },
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

/// A `MATCH … CREATE …` query produces a `SinglePartBody::Updating` with one
/// updating clause and no RETURN.
///
/// Unit: `parse()` / AST `SinglePartBody`
/// Precondition: `MATCH (a) CREATE (a)-[:KNOWS]->(b);`.
/// Expectation: `updating.len() == 1` and `return_clause.is_none()`.
#[test]
fn test_match_create_has_updating_body() {
    let query = parse("MATCH (a) CREATE (a)-[:KNOWS]->(b);").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            decypher::ast::query::SingleQueryKind::SinglePart(spq) => match &spq.body {
                SinglePartBody::Updating {
                    updating,
                    return_clause,
                } => {
                    check!(updating.len() == 1);
                    check!(return_clause.is_none());
                }
                _ => panic!("expected Updating body"),
            },
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

/// An `OPTIONAL MATCH` clause has `optional == true` on the `Match` AST node.
///
/// Unit: `parse()` / AST `Match::optional`
/// Precondition: `OPTIONAL MATCH (n) RETURN n;`.
/// Expectation: `m.optional == true`.
#[test]
fn test_optional_match_flag() {
    let query = parse("OPTIONAL MATCH (n) RETURN n;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            decypher::ast::query::SingleQueryKind::SinglePart(spq) => {
                match &spq.reading_clauses[0] {
                    decypher::ast::query::ReadingClause::Match(m) => {
                        check!(m.optional);
                    }
                    _ => panic!("expected Match clause"),
                }
            }
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

/// An `UNWIND … AS x` clause stores the binding variable name `"x"`.
///
/// Unit: `parse()` / AST `Unwind::variable`
/// Precondition: `UNWIND [1, 2, 3] AS x RETURN x;`.
/// Expectation: `u.variable.name.name == "x"`.
#[test]
fn test_unwind_has_expression_and_variable() {
    let query = parse("UNWIND [1, 2, 3] AS x RETURN x;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            decypher::ast::query::SingleQueryKind::SinglePart(spq) => {
                match &spq.reading_clauses[0] {
                    decypher::ast::query::ReadingClause::Unwind(u) => {
                        check!(u.variable.name.name == "x");
                    }
                    _ => panic!("expected Unwind clause"),
                }
            }
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

/// A node pattern `(n:Person)` has a bound variable `"n"` and one label.
///
/// Unit: `parse()` / AST `NodePattern`
/// Precondition: `MATCH (n:Person) RETURN n;`.
/// Expectation: The start node of the first path has `variable.name == "n"` and
///   `labels.len() == 1`.
#[test]
fn test_pattern_has_node() {
    let query = parse("MATCH (n:Person) RETURN n;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => {
            match &sq.kind {
                decypher::ast::query::SingleQueryKind::SinglePart(spq) => {
                    match &spq.reading_clauses[0] {
                        decypher::ast::query::ReadingClause::Match(m) => {
                            check!(m.pattern.parts.len() == 1);
                            let part = &m.pattern.parts[0];
                            // The node variable is inside the anonymous pattern part
                            match &part.anonymous.element {
                                decypher::ast::pattern::PatternElement::Path { start, .. } => {
                                    check!(start.variable.is_some());
                                    check!(start.variable.as_ref().unwrap().name.name == "n");
                                    check!(start.labels.len() == 1);
                                }
                                _ => panic!("expected Path pattern element"),
                            }
                        }
                        _ => panic!("expected Match clause"),
                    }
                }
                _ => panic!("expected SinglePart query"),
            }
        }
        _ => panic!("expected SingleQuery"),
    }
}

/// A `UNION` query is represented in the parsed `Query` statement list.
///
/// Unit: `parse()` / AST `RegularQuery::unions`
/// Precondition: Two MATCH/RETURN branches joined by `UNION`.
/// Expectation: `query.statements.len() >= 1`.
#[test]
fn test_union_has_two_queries() {
    let query =
        parse("MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;").unwrap();
    check!(query.statements.len() >= 1);
    // The UNION creates a RegularQuery with unions
    // Our current structure stores it in RegularQuery.unions
}

/// A parsed query's top-level span is a non-empty range.
///
/// Unit: `parse()` / AST `Query::span`
/// Precondition: `MATCH (n) RETURN n;` — non-empty input.
/// Expectation: `query.span.start < query.span.end`.
#[test]
fn test_span_is_nonzero() {
    let query = parse("MATCH (n) RETURN n;").unwrap();
    check!(query.span.start < query.span.end);
}

/// `range(start, end)` in expression position parses as an ordinary function
/// invocation — not an error, and not a bare variable followed by a stray
/// parenthesis. `range` is a contextual keyword (reserved only in the schema
/// `CREATE RANGE INDEX …` position), so as an expression it is the standard
/// openCypher list function.
///
/// Unit: `parse()` / AST `Expression::FunctionCall`
/// Precondition: `RETURN range(0, 3) AS r;`.
/// Expectation: the projection expression is a `FunctionCall` named `range`
/// with two arguments.
#[test]
fn test_range_is_a_function_invocation() {
    use decypher::ast::expr::Expression;

    let query = parse("RETURN range(0, 3) AS r;").unwrap();
    let QueryBody::SingleQuery(sq) = &query.statements[0] else {
        panic!("expected SingleQuery");
    };
    let decypher::ast::query::SingleQueryKind::SinglePart(spq) = &sq.kind else {
        panic!("expected SinglePart query");
    };
    let SinglePartBody::Return(ret) = &spq.body else {
        panic!("expected Return body");
    };
    match &ret.items[0].expression {
        Expression::FunctionCall(fi) => {
            check!(fi.name.len() == 1);
            check!(fi.name[0].name == "range");
            check!(fi.arguments.len() == 2);
        }
        other => panic!("expected a FunctionCall, got {other:?}"),
    }
}

/// A three-argument `range(start, end, step)` also parses (the optional step is
/// just another ordinary argument).
///
/// Unit: `parse()` / AST `Expression::FunctionCall`
/// Precondition: `RETURN range(0, 10, 2) AS r;`.
/// Expectation: a `FunctionCall` named `range` with three arguments.
#[test]
fn test_range_with_step_has_three_arguments() {
    use decypher::ast::expr::Expression;

    let query = parse("RETURN range(0, 10, 2) AS r;").unwrap();
    let QueryBody::SingleQuery(sq) = &query.statements[0] else {
        panic!("expected SingleQuery");
    };
    let decypher::ast::query::SingleQueryKind::SinglePart(spq) = &sq.kind else {
        panic!("expected SinglePart query");
    };
    let SinglePartBody::Return(ret) = &spq.body else {
        panic!("expected Return body");
    };
    match &ret.items[0].expression {
        Expression::FunctionCall(fi) => {
            check!(fi.arguments.len() == 3);
        }
        other => panic!("expected a FunctionCall, got {other:?}"),
    }
}

/// `range` remains usable as an ordinary variable name (it is only a contextual
/// keyword). Adding it to the expression function-call arm must not regress
/// this.
///
/// Unit: `parse()`
/// Precondition: `WITH 1 AS range RETURN range;`.
/// Expectation: parser returns `Ok`.
#[test]
fn test_range_is_still_a_valid_variable_name() {
    let result = parse("WITH 1 AS range RETURN range;");
    check!(result.is_ok(), "{:?}", result.err());
}

/// Asserts that `input` parses and prints back as `expected`.
fn check_roundtrip(input: &str, expected: &str) {
    use decypher::ast::ToCypher;
    let query = parse(input).unwrap_or_else(|e| panic!("failed to parse {input:?}: {e}"));
    check!(query.to_cypher() == expected, "input: {input}");
}

/// Variable references are kept as function arguments (#44, #42, #43).
///
/// Unit: `parse()` / AST `FunctionInvocation`
/// Precondition: Function calls whose arguments are bare variables, mixed
///               with binary expressions, nested calls, and a trailing index.
/// Expectation: Every argument survives the round trip.
#[test]
fn test_variable_function_arguments_are_kept() {
    check_roundtrip(
        "WITH 42 AS s1 RETURN abs(s1)",
        "WITH 42 AS s1 RETURN abs(s1);",
    );
    check_roundtrip(
        "WITH 1 AS a, 2 AS b RETURN coalesce(a, b + 1, a * b) AS r",
        "WITH 1 AS a, 2 AS b RETURN coalesce(a, b + 1, a * b) AS r;",
    );
    check_roundtrip(
        "MATCH (n:Person) RETURN labels(n)[0] AS first_label LIMIT 1",
        "MATCH (n:Person) RETURN labels(n)[0] AS first_label LIMIT 1;",
    );
    check_roundtrip(
        "MATCH (x) RETURN head(labels(x)) AS h, x",
        "MATCH (x) RETURN head(labels(x)) AS h, x;",
    );
}

/// List comprehensions keep their collection and `WHERE` filter, including
/// an `IN` inside the filter (#41).
///
/// Unit: `parse()` / AST `ListComprehension`
/// Precondition: Comprehensions over variables and binary expressions.
/// Expectation: Collection, filter and map all survive the round trip.
#[test]
fn test_list_comprehension_keeps_collection_and_filter() {
    check_roundtrip(
        "WITH [1, 2] AS s1, [2] AS s2 RETURN [x IN s1 WHERE x IN s2] AS i",
        "WITH [1, 2] AS s1, [2] AS s2 RETURN [x IN s1 WHERE x IN s2] AS i;",
    );
    check_roundtrip(
        "WITH [1] AS a RETURN [x IN a + [2] WHERE x > 0 | x * 2] AS r",
        "WITH [1] AS a RETURN [x IN a + [2] WHERE x > 0 | x * 2] AS r;",
    );
    check_roundtrip("RETURN [x IN [1, 2]] AS r", "RETURN [x IN [1, 2]] AS r;");
}

/// `all`/`any`/`none`/`single` calls build quantifier expressions (#41).
///
/// Unit: `parse()` / AST `Expression::{All, Any, None, Single}`
/// Precondition: Quantifier calls with and without `WHERE`.
/// Expectation: Variable, collection and predicate survive the round trip.
#[test]
fn test_quantifiers_keep_variable_and_collection() {
    check_roundtrip(
        "WITH [1] AS s RETURN any(y IN s WHERE y = 1) AS r",
        "WITH [1] AS s RETURN ANY(y IN s WHERE y = 1) AS r;",
    );
    check_roundtrip(
        "RETURN all(y IN [1] WHERE y > 0), none(y IN [1] WHERE y < 0), single(y IN [1])",
        "RETURN ALL(y IN [1] WHERE y > 0), NONE(y IN [1] WHERE y < 0), SINGLE(y IN [1]);",
    );
}

/// `reduce` keeps its accumulator, list and expression (#45).
///
/// Unit: `parse()` / AST `Expression::Reduce`
/// Precondition: `reduce` with a binary list and a binary expression.
/// Expectation: All parts survive the round trip.
#[test]
fn test_reduce_round_trip() {
    check_roundtrip(
        "WITH [1] AS l RETURN reduce(acc = 0, x IN l + [2] | acc + x * 2) AS t",
        "WITH [1] AS l RETURN reduce(acc = 0, x IN l + [2] | acc + x * 2) AS t;",
    );
}

/// Updating clauses after the last `WITH` are kept, with or without a
/// trailing `RETURN` (#44).
///
/// Unit: `parse()` / AST multi-part final part
/// Precondition: `WITH` followed by `FOREACH` over a variable, and by
///               `CREATE … RETURN`.
/// Expectation: The updating clauses survive the round trip.
#[test]
fn test_multi_part_final_updating_clauses_are_kept() {
    check_roundtrip(
        "MATCH (n) WITH collect(n) AS ns FOREACH (m IN ns | DETACH DELETE m)",
        "MATCH (n) WITH collect(n) AS ns FOREACH (m IN ns | DETACH DELETE m);",
    );
    check_roundtrip(
        "MATCH (n) WITH n CREATE (m:M) RETURN m",
        "MATCH (n) WITH n CREATE (m:M) RETURN m;",
    );
}

/// Slice bounds made of binary expressions keep the composed expression.
///
/// Unit: `parse()` / AST `Expression::ListSlice`
/// Precondition: `l[a + 1..b * 2]`.
/// Expectation: Both bounds survive the round trip.
#[test]
fn test_list_slice_binary_bounds() {
    check_roundtrip(
        "WITH [1] AS l, 0 AS a, 1 AS b RETURN l[a + 1..b * 2] AS r",
        "WITH [1] AS l, 0 AS a, 1 AS b RETURN l[a + 1..b * 2] AS r;",
    );
}

/// Composed expressions in comma-separated lists and single-expression slots
/// keep their full composition instead of only the first node of the flat CST
/// chain (#56, #58, #59).
///
/// Unit: `parse()` / CST accessors over flat binary chains
/// Precondition: Binary, comparison, property and index chains in map values,
///               list elements, `DELETE`, `UNWIND`, `SKIP`/`LIMIT`,
///               `LOAD CSV`, map projections and dynamic property keys.
/// Expectation: Every expression survives the round trip unchanged.
#[test]
fn test_flat_chains_are_kept_whole() {
    for q in [
        "RETURN {a: 1 + 2, b: 1 < 2, c: true AND false} AS m",
        "MATCH (n {flag: 1 = $x}) RETURN n",
        "MATCH (a) RETURN [a.l[1], a.l[0]] AS x",
        "MATCH (a) WITH a ORDER BY [a.l[1]] + a.l RETURN a",
        "MATCH (u) WITH {key: collect(u)} AS m DETACH DELETE m.key[0], u",
        "UNWIND [[1]] AS l WITH l UNWIND l AS x RETURN x",
        "UNWIND a.b[0] AS x RETURN x",
        "MATCH (n) RETURN n SKIP $s + 1 LIMIT $l * 2",
        "LOAD CSV FROM $base + 'x.csv' AS line RETURN line",
        "MATCH (n) RETURN n{.a, b: n.x + 1} AS m",
        "MATCH (n) SET n[$k + 'x'] = 1 RETURN n",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
}

/// A parenthesis is only read as a relationship pattern when a relationship
/// actually follows it, so parenthesised arguments and operands survive (#60).
///
/// Unit: `parse()` / pattern-vs-parenthesis lookahead
/// Precondition: Parenthesised operands followed by arithmetic or comparison
///               operators that share prefixes with ASCII arrows, and
///               `exists(...)` around real patterns.
/// Expectation: Each query round-trips with its original meaning.
#[test]
fn test_parenthesis_vs_relationship_pattern() {
    for q in [
        "RETURN a - f((b)) - c",
        "RETURN (1) - 2 AS b",
        "WITH 1 AS a RETURN (a) - 1 AS b",
        "WITH 1 AS a RETURN (a) - -1 AS b",
        "WITH 1 AS a RETURN (a) < -1 AS b",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
    check_roundtrip(
        "MATCH (n) WHERE exists((n)<--()) RETURN n",
        "MATCH (n) WHERE EXISTS { (n)<-[]-() } RETURN n;",
    );
    check_roundtrip(
        "MATCH (a {x: (1)})-->(b) RETURN a",
        "MATCH (a {x: (1)})-[]->(b) RETURN a;",
    );
}

/// Procedure calls keep their arguments, the `CALL` keyword and their
/// `YIELD` items, and an in-query call is kept as a reading clause (#57).
///
/// Unit: `parse()` / procedure calls
/// Precondition: Standalone calls with explicit and implicit arguments, and
///               in-query calls with `YIELD … WHERE`, aliases and more clauses.
/// Expectation: Each query round-trips unchanged.
#[test]
fn test_procedure_calls_round_trip() {
    check_roundtrip("CALL foo(1, 2)", "CALL foo(1, 2);");
    check_roundtrip("CALL foo", "CALL foo;");
    check_roundtrip("CALL test.my.proc YIELD *", "CALL test.my.proc YIELD *;");
    for q in [
        "CALL db.labels() YIELD label",
        "CALL foo() YIELD x RETURN x",
        "MATCH (n) CALL foo(n, 1 + 2) YIELD x WHERE x > 1 RETURN x",
        "CALL test.my.proc(NULL) YIELD a AS c, b RETURN c, b",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
}

/// Relationship patterns used as expressions and pattern comprehensions keep
/// their full pattern, predicate and projection (#61, #62).
///
/// Unit: `parse()` / pattern expressions and pattern comprehensions
/// Precondition: Patterns as `WHERE` predicates, function arguments and
///               projections, and pattern comprehensions with a path variable,
///               labels, properties, `WHERE` and a composed map expression.
/// Expectation: Each query round-trips with the pattern intact.
#[test]
fn test_pattern_expressions_and_comprehensions_round_trip() {
    for (input, printed) in [
        (
            "MATCH (a) WHERE (a)-[:T]->(:L) RETURN a",
            "MATCH (a) WHERE (a)-[:T]->(:L) RETURN a;",
        ),
        (
            "MATCH (n) WHERE NOT (n)-->() RETURN n",
            "MATCH (n) WHERE NOT (n)-[]->() RETURN n;",
        ),
        (
            "MATCH (n) WHERE (n)-[r:REL*0..2]->(m {num: 5}) RETURN n",
            "MATCH (n) WHERE (n)-[r:REL*0..2]->(m {num: 5}) RETURN n;",
        ),
        (
            "MATCH (a) RETURN size((a)-->()) AS n",
            "MATCH (a) RETURN size((a)-[]->()) AS n;",
        ),
        (
            "MATCH (a) RETURN [(a)-->(b) | b] AS x",
            "MATCH (a) RETURN [(a)-[]->(b) | b] AS x;",
        ),
        (
            "MATCH (a) RETURN [p = (a:L {x: 1})-[r:T]->(b) WHERE b.x > 1 | b.name + 'x'] AS x",
            "MATCH (a) RETURN [p = (a:L {x: 1})-[r:T]->(b) WHERE b.x > 1 | b.name + 'x'] AS x;",
        ),
    ] {
        check_roundtrip(input, printed);
    }
}

/// Label items in `SET`/`REMOVE` keep their variable and every label (#77).
///
/// Unit: `parse()` / `SetItem::Labels`, `RemoveItem::Labels`
/// Precondition: Single and multiple labels, mixed with property items, in
///               `SET`, `REMOVE` and `MERGE … ON MATCH/ON CREATE SET`.
/// Expectation: Each query round-trips unchanged.
#[test]
fn test_label_items_keep_variable_and_labels() {
    for q in [
        "MATCH (n) SET n:Foo RETURN n",
        "MATCH (n) SET n:Foo:Bar, n.x = 1 RETURN n",
        "MATCH (n) REMOVE n:A:B RETURN n",
        "MATCH (n) REMOVE n:`My Label`, n.x RETURN n",
        "MERGE (a) ON MATCH SET a:M1 ON CREATE SET a:M2",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
}

/// `EXISTS { pattern WHERE … }` keeps its pattern and predicate (#78), and
/// parameter and identifier names print in a form that parses back to the
/// same name (#66).
///
/// Unit: `parse()` / `ExistsExpression`, `Parameter`, `SymbolicName` printing
/// Precondition: Pattern-form `exists` with `WHERE`, nested in a subquery;
///               numeric, escaped and plain parameters; names containing
///               backticks.
/// Expectation: Each query round-trips with the same names and content.
#[test]
fn test_exists_patterns_and_names_round_trip() {
    check_roundtrip(
        "MATCH (n) WHERE exists { (n)-->(m) WHERE n.p = m.p } RETURN n",
        "MATCH (n) WHERE EXISTS { (n)-[]->(m) WHERE n.p = m.p } RETURN n;",
    );
    check_roundtrip(
        "MATCH (n) WHERE exists { MATCH (m) WHERE exists { (n)-[]->(m) WHERE n.prop = m.prop } RETURN true } RETURN n",
        "MATCH (n) WHERE EXISTS { MATCH (m) WHERE EXISTS { (n)-[]->(m) WHERE n.prop = m.prop } RETURN true } RETURN n;",
    );
    for q in [
        "RETURN $1 AS a, $`odd param` AS b, $p AS c",
        "MATCH (n:`we``ird`) RETURN n.`a``b` AS `x``y`",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
}

/// Map projection variable selectors stay variable selectors, and
/// `FIELDTERMINATOR` is kept (#74).
///
/// Unit: `parse()` / `MapProjectionItem::Variable`, `LoadCsv::field_terminator`
/// Precondition: A map projection mixing `.prop`, a variable selector and
///               `.*`; `LOAD CSV` with and without `FIELDTERMINATOR`.
/// Expectation: Each query round-trips unchanged.
#[test]
fn test_map_projection_variables_and_field_terminator() {
    for q in [
        "MATCH (n) WITH n, 1 AS c RETURN n{.a, c, .*} AS m",
        "LOAD CSV WITH HEADERS FROM 'f' AS line FIELDTERMINATOR ';' RETURN line",
        "LOAD CSV FROM 'f' AS line RETURN line",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
}

/// A list starting with `(` or `-` is only a pattern comprehension when a
/// relationship pattern follows; otherwise it is a list literal.
///
/// Unit: `parse()` / bracket disambiguation
/// Precondition: Lists starting with a negative number, a negated variable,
///               a parenthesised expression, or a comparison `x = 1`, next to
///               real pattern comprehensions.
/// Expectation: Each query round-trips unchanged.
#[test]
fn test_bracket_list_vs_pattern_comprehension() {
    for q in [
        "RETURN [-1000, -3]",
        "WITH 1 AS x RETURN [-x]",
        "RETURN [(1), 2]",
        "RETURN [(1 + 2) * 3]",
        "WITH 1 AS x RETURN [x = 1]",
    ] {
        check_roundtrip(q, &format!("{q};"));
    }
    check_roundtrip(
        "MATCH (a) RETURN [p = (a)-->(b) | p] AS x",
        "MATCH (a) RETURN [p = (a)-[]->(b) | p] AS x;",
    );
}

/// Number literals: octal, leading-dot floats and `i64::MIN` parse (#63).
///
/// Unit: `parse()` / number literals
/// Precondition: `0o` and legacy octal, floats without a leading zero, and
///               the minimum 64-bit integer in decimal and hex.
/// Expectation: Each parses to the expected value.
#[test]
fn test_number_literals() {
    for (input, printed) in [
        ("RETURN 0o17, -0o17, 017", "RETURN 15, -15, 15;"),
        ("RETURN .5, -.5e-3", "RETURN 0.5, -0.0005;"),
        ("RETURN [-.1e-5]", "RETURN [-1e-6];"),
        (
            "RETURN -9223372036854775808",
            "RETURN -9223372036854775808;",
        ),
        ("RETURN -0x8000000000000000", "RETURN -9223372036854775808;"),
        (
            "WITH [1, 2, 3] AS l RETURN l[1..2]",
            "WITH [1, 2, 3] AS l RETURN l[1..2];",
        ),
    ] {
        check_roundtrip(input, printed);
    }
}

/// Non-reserved keywords can be used as variables, and the reserved literals
/// as map keys (#64).
///
/// Unit: `parse()` / keyword-as-name handling
/// Precondition: Variables named `node`, `types`, `rows`, `match` and `key`
///               (including as a list comprehension variable), and map keys
///               `null`, `true`, `false`.
/// Expectation: Each query parses and prints back with the same names.
#[test]
fn test_keywords_as_names() {
    for (input, printed) in [
        ("RETURN node", "RETURN node;"),
        (
            "WITH 1 AS node RETURN node.x",
            "WITH 1 AS node RETURN node.x;",
        ),
        (
            "WITH 1 AS rows RETURN size(rows)",
            "WITH 1 AS rows RETURN size(rows);",
        ),
        (
            "WITH 1 AS match RETURN match",
            "WITH 1 AS `match` RETURN `match`;",
        ),
        (
            "WITH [1] AS k RETURN [key IN k | key]",
            "WITH [1] AS k RETURN [key IN k | key];",
        ),
        (
            "RETURN {null: 1, true: 2, false: 3} AS m",
            "RETURN {`null`: 1, `true`: 2, `false`: 3} AS m;",
        ),
    ] {
        check_roundtrip(input, printed);
    }
}

/// Pattern parameters, legacy relationship-type alternation, parenthesised
/// `SET` targets and keyword labels in label predicates (#65).
///
/// Unit: `parse()`
/// Precondition: `(n $param)`, `[r:FOO $param]`, `[:A|:B]`, `SET (n).name`,
///               `RETURN m:TYPE` and `RETURN (-3) - 2`.
/// Expectation: Each query parses; legacy `|:` prints in the modern form.
#[test]
fn test_misc_valid_forms() {
    for (input, printed) in [
        ("MATCH (n $param) RETURN n", "MATCH (n $param) RETURN n;"),
        (
            "MATCH ()-[r:FOO $param]->() RETURN r",
            "MATCH ()-[r:FOO $param]->() RETURN r;",
        ),
        (
            "MATCH (a)-[:A|:B]->(b) RETURN b",
            "MATCH (a)-[:A|B]->(b) RETURN b;",
        ),
        (
            "MATCH (n:A) SET (n).name = 'neo4j' RETURN n",
            "MATCH (n:A) SET (n).name = 'neo4j' RETURN n;",
        ),
        ("MATCH (n) RETURN n:TYPE", "MATCH (n) RETURN n:TYPE;"),
        ("RETURN (-3) - 2 AS b", "RETURN (-3) - 2 AS b;"),
    ] {
        check_roundtrip(input, printed);
    }
}

/// GQL / Neo4j 5 forms: `UNION DISTINCT` and `IS Label` label predicates in
/// expressions and patterns (#69). They print in the equivalent openCypher
/// form.
///
/// Unit: `parse()`
/// Precondition: `UNION DISTINCT` at top level and in a subquery; `IS` label
///               predicates in WHERE and RETURN, in node patterns and in
///               relationship details; existing `IS NULL` forms.
/// Expectation: Each parses and prints the equivalent openCypher query.
#[test]
fn test_gql_union_distinct_and_is_label() {
    for (input, printed) in [
        (
            "RETURN 1 UNION DISTINCT RETURN 2",
            "RETURN 1 UNION RETURN 2;",
        ),
        (
            "CALL { RETURN 1 AS a UNION DISTINCT RETURN 2 AS a } RETURN a",
            "CALL { RETURN 1 AS a UNION RETURN 2 AS a } RETURN a;",
        ),
        (
            "MATCH (n) WHERE n IS Person RETURN n",
            "MATCH (n) WHERE n:Person RETURN n;",
        ),
        (
            "MATCH (n) RETURN n IS Person AS p",
            "MATCH (n) RETURN n:Person AS p;",
        ),
        (
            "MATCH (n IS Person)-[r IS KNOWS]->(m) RETURN r",
            "MATCH (n:Person)-[r:KNOWS]->(m) RETURN r;",
        ),
        (
            "MATCH (n) WHERE n.x IS NOT NULL AND n:A IS NULL RETURN n",
            "MATCH (n) WHERE n.x IS NOT NULL AND n:A IS NULL RETURN n;",
        ),
    ] {
        check_roundtrip(input, printed);
    }
}
