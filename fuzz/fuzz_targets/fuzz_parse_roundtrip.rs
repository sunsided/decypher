#![no_main]

//! Printing a parsed query and parsing the result gives back the same AST.
//!
//! Comparing the trees, not just the printed text, matters: a printer that
//! dropped a needed pair of parentheses would print a query that means
//! something else, and that query would print the same way again.

use decypher::ast::ToCypher;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

/// The AST as JSON without source positions and source text, which printing
/// changes legitimately (whitespace, keyword case).
fn shape(query: &decypher::Query) -> Value {
    fn strip(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.retain(|key, _| !key.ends_with("span") && key != "expression_text");
                fields.values_mut().for_each(strip);
            }
            Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(query).expect("the AST serialises");
    strip(&mut value);
    value
}

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(ast) = decypher::parse(source) else {
        return;
    };
    // shortcut: schema commands are only partly parsed (their unparsed tail
    // is skipped), so their AST cannot round-trip yet; check them once the
    // schema-command grammar is complete.
    if ast
        .statements
        .iter()
        .any(|s| matches!(s, decypher::ast::query::QueryBody::SchemaCommand(_)))
    {
        return;
    }
    let printed = ast.to_cypher();
    let reparsed = decypher::parse(printed.as_str()).unwrap_or_else(|e| {
        panic!("the printed query does not parse: {e}\n  input:   {source:?}\n  printed: {printed:?}")
    });
    assert_eq!(
        shape(&ast),
        shape(&reparsed),
        "printing changed the AST\n  input:   {source:?}\n  printed: {printed:?}"
    );
});
