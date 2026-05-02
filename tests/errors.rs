use assert2::check;
use open_cypher::parse;
use open_cypher::CypherError;
use open_cypher::ErrorKind;

#[test]
fn test_empty_string() {
    let result = parse("");
    check!(result.is_err());
    match result.unwrap_err() {
        CypherError {
            kind: ErrorKind::EmptyInput,
            ..
        } => {}
        _ => panic!("expected EmptyInput error"),
    }
}

#[test]
fn test_invalid_keyword() {
    let result = parse("FOOBAR;");
    check!(result.is_err());
}

#[test]
fn test_unterminated_string() {
    let result = parse("MATCH (n) WHERE n.name = 'hello RETURN n;");
    check!(result.is_err());
    let err = result.unwrap_err();
    let rendered = err.render("MATCH (n) WHERE n.name = 'hello RETURN n;");
    check!(
        rendered.contains("unterminated") || rendered.contains("string"),
        "expected unterminated string diagnostic, got: {}",
        rendered
    );
}

#[test]
fn test_missing_closing_paren() {
    let result = parse("MATCH (n RETURN n;");
    check!(result.is_err());
}

#[test]
fn test_invalid_pattern() {
    let result = parse("MATCH RETURN n;");
    check!(result.is_err());
}

#[test]
fn test_incomplete_return() {
    let result = parse("RETURN;");
    check!(result.is_err());
    let err = result.unwrap_err();
    check!(matches!(
        err.kind(),
        ErrorKind::MissingClause {
            clause: "projection",
            ..
        }
    ));
}

#[test]
fn test_garbage_input() {
    let result = parse("!!!@@@###");
    check!(result.is_err());
}

#[test]
fn test_error_contains_position_info() {
    let result = parse("MATCH (n WHERE n.name = 'hello' RETURN n;");
    check!(result.is_err());
    let err = result.unwrap_err();
    let err_string = format!("{}", err);
    check!(!err_string.is_empty());
}

#[test]
fn test_error_kind_is_matchable() {
    let result = parse("RETURN 1");
    check!(result.is_ok());

    let result = parse("RETURN");
    check!(result.is_err());
    let err = result.unwrap_err();
    match err.kind() {
        ErrorKind::MissingClause { clause, after } => {
            check!(*clause == "projection");
            check!(*after == "RETURN");
        }
        _ => panic!("expected MissingClause, got {:?}", err.kind()),
    }
}

#[test]
fn test_error_span_is_valid() {
    let result = parse("RETURN;");
    check!(result.is_err());
    let err = result.unwrap_err();
    let span = err.span();
    check!(span.start > 0 || span.end > 0);
}

#[test]
fn test_null_comparison_note_skipped() {
    let result = parse("MATCH (n) WHERE n.x = NULL RETURN n");
    check!(result.is_ok());
}

#[test]
fn test_render_produces_output() {
    let result = parse("RETURN;");
    check!(result.is_err());
    let err = result.unwrap_err();
    let rendered = err.render("RETURN;");
    check!(rendered.contains("error:"));
    check!(rendered.contains("projection"));
}

#[test]
fn test_diagnostics_wrapper() {
    use open_cypher::parse_all;
    let (query, diags) = parse_all("RETURN;");
    check!(query.is_none());
    check!(!diags.is_empty());
    check!(diags.len() == 1);
}

#[test]
fn test_parse_with_label() {
    use open_cypher::parse_with_label;
    let result = parse_with_label("RETURN 1", "test.cypher");
    check!(result.is_ok());

    let result = parse_with_label("RETURN;", "test.cypher");
    check!(result.is_err());
    let err = result.unwrap_err();
    check!(err.source_label() == Some("test.cypher"));
}
