use cypher::ast::ToCypher;
use cypher::cst::AstNode;
use cypher::{cst, parse};
use proptest::prelude::*;

fn ident() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,5}".prop_map(|s| format!("v{s}"))
}

fn string_lit() -> impl Strategy<Value = String> {
    "[A-Za-z0-9]{0,8}".prop_map(|s| format!("'{s}'"))
}

fn int_lit() -> impl Strategy<Value = String> {
    (0i64..1000).prop_map(|n| n.to_string())
}

fn parameter() -> impl Strategy<Value = String> {
    ident().prop_map(|name| format!("${name}"))
}

fn property_expr() -> impl Strategy<Value = String> {
    (ident(), ident()).prop_map(|(base, key)| format!("{base}.{key}"))
}

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

fn label_atom() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("Person")),
        Just(String::from("Company")),
        Just(String::from("Deleted")),
        ident().prop_map(|name| format!("$({name})")),
    ]
}

fn label_expr() -> impl Strategy<Value = String> {
    label_atom().prop_recursive(4, 32, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|s| format!("!{s}")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}|{b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}&{b})")),
        ]
    })
}

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

fn return_query() -> impl Strategy<Value = String> {
    expr().prop_map(|e| format!("RETURN {e};"))
}

fn return_query_stable() -> impl Strategy<Value = String> {
    expr_stable().prop_map(|e| format!("RETURN {e};"))
}

fn rich_match_query() -> impl Strategy<Value = String> {
    (label_expr(), ident())
        .prop_map(|(labels, var)| format!("MATCH ({var}:{labels}) RETURN {var};"))
}

fn postfix_label_query() -> impl Strategy<Value = String> {
    (ident(), label_expr())
        .prop_map(|(var, labels)| format!("MATCH ({var}) WHERE {var}:{labels} RETURN {var};"))
}

fn quantified_path_query() -> impl Strategy<Value = String> {
    (label_expr(), label_expr(), quantifier()).prop_map(|(left, right, q)| {
        format!("MATCH p = ((a:{left})-[:LINK]->(b:{right})){q} RETURN p;")
    })
}

fn quantified_relationship_query() -> impl Strategy<Value = String> {
    (label_expr(), label_expr(), quantifier()).prop_map(|(left, right, q)| {
        format!("MATCH p = (a:{left})-[:LINK]->{q}(b:{right}) RETURN p;")
    })
}

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

    #[test]
    fn prop_generated_queries_roundtrip_ast(query in ast_roundtrip_query()) {
        let parsed = parse(&query).unwrap_or_else(|err| panic!("AST parse failed for `{query}`: {err}"));
        let emitted = parsed.to_cypher();
        let reparsed = parse(&emitted)
            .unwrap_or_else(|err| panic!("AST reparse failed for emitted `{emitted}` from `{query}`: {err}"));
        prop_assert_eq!(parsed, reparsed);
    }
}
