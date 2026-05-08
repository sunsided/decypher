//! Property-based tests for the Cypher parser.
//!
//! These tests use `proptest` to generate random but syntactically valid
//! Cypher fragments and verify that:
//! - All generated queries parse losslessly via the CST path.
//! - Round-tripping (parse → `to_cypher` → re-parse) preserves the AST.

use decypher::ast::ToCypher;
use decypher::cst::AstNode;
use decypher::{cst, parse};
use proptest::prelude::*;

/// Generate a simple valid identifier of the form `v[a-z][a-z0-9]{0,5}`.
fn ident() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,5}".prop_map(|s| format!("v{s}"))
}

/// Generate a single-quoted string literal with alphanumeric content.
fn string_lit() -> impl Strategy<Value = String> {
    "[A-Za-z0-9]{0,8}".prop_map(|s| format!("'{s}'"))
}

/// Generate a small non-negative integer literal (0 to 999).
fn int_lit() -> impl Strategy<Value = String> {
    (0i64..1000).prop_map(|n| n.to_string())
}

/// Generate a `$identifier` parameter reference.
fn parameter() -> impl Strategy<Value = String> {
    ident().prop_map(|name| format!("${name}"))
}

/// Generate a property-access expression `base.key`.
fn property_expr() -> impl Strategy<Value = String> {
    (ident(), ident()).prop_map(|(base, key)| format!("{base}.{key}"))
}

/// Generate a scalar atom (literal, identifier, parameter, or property).
fn scalar_atom() -> impl Strategy<Value = String> {
    prop_oneof![
        int_lit(),
        string_lit(),
        Just(String::from("true")),
        Just(String::from("false")),
        Just(String::from("null")),
        ident(),
        parameter(),
        property_expr(),
    ]
}

/// Like [`scalar_atom`] but without `property_expr`, for stable round-trip tests
/// where property access aliases cannot be inferred.
fn scalar_atom_stable() -> impl Strategy<Value = String> {
    prop_oneof![
        int_lit(),
        string_lit(),
        Just(String::from("true")),
        Just(String::from("false")),
        Just(String::from("null")),
        ident(),
        parameter(),
    ]
}

/// Generate a recursive expression tree.
fn expr() -> impl Strategy<Value = String> {
    let leaf = scalar_atom();
    leaf.prop_recursive(4, 32, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} + {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} = {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} AND {b})")),
            inner.clone().prop_map(|a| format!("NOT {a}")),
            proptest::collection::vec(scalar_atom(), 0..4)
                .prop_map(|items| format!("[{}]", items.join(", "))),
        ]
    })
}

/// Like [`expr`] but using [`scalar_atom_stable`] leaves for stable round-trip tests.
fn expr_stable() -> impl Strategy<Value = String> {
    let leaf = scalar_atom_stable();
    leaf.prop_recursive(4, 32, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} + {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} = {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} AND {b})")),
            inner.clone().prop_map(|a| format!("NOT {a}")),
            proptest::collection::vec(scalar_atom_stable(), 0..4)
                .prop_map(|items| format!("[{}]", items.join(", "))),
        ]
    })
}

/// Generate a label atom (named label or dynamic `$(var)` label).
fn label_atom() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("Person")),
        Just(String::from("Company")),
        Just(String::from("Deleted")),
        ident().prop_map(|name| format!("$({name})")),
    ]
}

/// Generate a recursive label expression (NOT, OR `|`, AND `&`).
fn label_expr() -> impl Strategy<Value = String> {
    label_atom().prop_recursive(4, 32, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|s| format!("!{s}")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}|{b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}&{b})")),
        ]
    })
}

/// Generate a valid quantifier string (`{n}`, `{n,m}`, `{n,}`, `{,m}`).
fn quantifier() -> impl Strategy<Value = String> {
    prop_oneof![
        (1u8..4).prop_map(|n| format!("{{{n}}}")),
        (1u8..4, 1u8..4).prop_map(|(a, b)| {
            let (start, mut end) = if a <= b { (a, b) } else { (b, a) };
            if start == end {
                end += 1;
            }
            format!("{{{start},{end}}}")
        }),
        (1u8..4).prop_map(|n| format!("{{{n},}}")),
        (1u8..4).prop_map(|m| format!("{{,{m}}}")),
    ]
}

/// Generate a `RETURN <expr>;` query string.
fn return_query() -> impl Strategy<Value = String> {
    expr().prop_map(|e| format!("RETURN {e};"))
}

/// Like [`return_query`] but using stable expressions suitable for round-trip tests.
fn return_query_stable() -> impl Strategy<Value = String> {
    expr_stable().prop_map(|e| format!("RETURN {e};"))
}

/// Generate `MATCH (var:labelExpr) RETURN var;` queries.
fn rich_match_query() -> impl Strategy<Value = String> {
    (label_expr(), ident())
        .prop_map(|(labels, var)| format!("MATCH ({var}:{labels}) RETURN {var};"))
}

/// Generate `MATCH (var) WHERE var:labelExpr RETURN var;` queries.
fn postfix_label_query() -> impl Strategy<Value = String> {
    (ident(), label_expr())
        .prop_map(|(var, labels)| format!("MATCH ({var}) WHERE {var}:{labels} RETURN {var};"))
}

/// Generate quantified-path pattern queries.
fn quantified_path_query() -> impl Strategy<Value = String> {
    (label_expr(), label_expr(), quantifier()).prop_map(|(left, right, q)| {
        format!("MATCH p = ((a:{left})-[:LINK]->(b:{right})){q} RETURN p;")
    })
}

/// Generate quantified-relationship queries.
fn quantified_relationship_query() -> impl Strategy<Value = String> {
    (label_expr(), label_expr(), quantifier()).prop_map(|(left, right, q)| {
        format!("MATCH p = (a:{left})-[:LINK]->{q}(b:{right}) RETURN p;")
    })
}

/// Generate `COUNT { … }`, `COLLECT { … }`, and `EXISTS { … }` subquery queries.
fn subquery_query() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("RETURN COUNT { MATCH (n:Person) RETURN n };")),
        Just(String::from(
            "RETURN COLLECT { MATCH (n:Person) RETURN n.name };"
        )),
        Just(String::from(
            "MATCH (p:Person) WHERE EXISTS { MATCH (p)-[:KNOWS]->(:Person) } RETURN p;"
        )),
    ]
}

/// Combine all valid-query strategies into one.
fn valid_query() -> impl Strategy<Value = String> {
    prop_oneof![
        return_query(),
        rich_match_query(),
        postfix_label_query(),
        quantified_path_query(),
        quantified_relationship_query(),
        subquery_query(),
    ]
}

/// Subset of valid queries suitable for AST round-trip testing.
fn ast_roundtrip_query() -> impl Strategy<Value = String> {
    prop_oneof![
        return_query_stable(),
        rich_match_query(),
        postfix_label_query(),
        quantified_path_query(),
        quantified_relationship_query(),
        Just(String::from("RETURN COUNT { MATCH (n:Person) RETURN n };")),
        Just(String::from(
            "RETURN COLLECT { MATCH (n:Person) RETURN n.name };"
        )),
    ]
}

proptest! {
    /// All randomly generated valid queries parse losslessly via the CST path.
    ///
    /// Unit: `cst::parse()`
    /// Precondition: Query generated by `valid_query()` strategy.
    /// Expectation: No CST parse errors; CST text round-trips to the original string.
    #[test]
    fn prop_generated_queries_parse_losslessly(query in valid_query()) {
        let parsed = cst::parse(&query);
        prop_assert!(
            parsed.errors.is_empty(),
            "CST parse errors for query `{}`: {:?}",
            query,
            parsed.errors
        );
        prop_assert_eq!(parsed.tree().syntax().text().to_string(), query);
    }

    /// Round-trip: parse → `to_cypher` → re-parse must yield equal ASTs.
    ///
    /// Unit: `Todecypher::to_cypher` / `parse()`
    /// Precondition: Query generated by `ast_roundtrip_query()` strategy.
    /// Expectation: Re-parsed AST equals the originally parsed AST.
    #[test]
    fn prop_generated_queries_roundtrip_ast(query in ast_roundtrip_query()) {
        let parsed = parse(&query).unwrap_or_else(|err| panic!("AST parse failed for `{query}`: {err}"));
        let emitted = parsed.to_cypher();
        let reparsed = parse(&emitted)
            .unwrap_or_else(|err| panic!("AST reparse failed for emitted `{emitted}` from `{query}`: {err}"));
        prop_assert_eq!(parsed, reparsed);
    }
}
