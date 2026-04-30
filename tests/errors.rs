use open_cypher::parse;
use open_cypher::CypherError;
use open_cypher::ErrorKind;

#[test]
fn test_empty_string() {
    let result = parse("");
    assert!(result.is_err());
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
    assert!(result.is_err());
}

#[test]
fn test_unterminated_string() {
    let result = parse("MATCH (n) WHERE n.name = 'hello RETURN n;");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let rendered = err.render("MATCH (n) WHERE n.name = 'hello RETURN n;");
    assert!(
        rendered.contains("unterminated") || rendered.contains("string"),
        "expected unterminated string diagnostic, got: {}",
        rendered
    );
}

#[test]
fn test_missing_closing_paren() {
    let result = parse("MATCH (n RETURN n;");
    assert!(result.is_err());
}

#[test]
fn test_invalid_pattern() {
    let result = parse("MATCH RETURN n;");
    assert!(result.is_err());
}

#[test]
fn test_incomplete_return() {
    let result = parse("RETURN;");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
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
    assert!(result.is_err());
}

#[test]
fn test_error_contains_position_info() {
    let result = parse("MATCH (n WHERE n.name = 'hello' RETURN n;");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_string = format!("{}", err);
    assert!(!err_string.is_empty());
}

#[test]
fn test_error_kind_is_matchable() {
    let result = parse("RETURN 1");
    assert!(result.is_ok());

    let result = parse("RETURN");
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err.kind() {
        ErrorKind::MissingClause { clause, after } => {
            assert_eq!(*clause, "projection");
            assert_eq!(*after, "RETURN");
        }
        _ => panic!("expected MissingClause, got {:?}", err.kind()),
    }
}

#[test]
fn test_error_span_is_valid() {
    let result = parse("RETURN;");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let span = err.span();
    assert!(span.start > 0 || span.end > 0);
}

#[test]
fn test_null_comparison_note_skipped() {
    let result = parse("MATCH (n) WHERE n.x = NULL RETURN n");
    assert!(result.is_ok());
}

#[test]
fn test_render_produces_output() {
    let result = parse("RETURN;");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let rendered = err.render("RETURN;");
    assert!(rendered.contains("error:"));
    assert!(rendered.contains("projection"));
}

#[test]
fn test_diagnostics_wrapper() {
    use open_cypher::parse_all;
    let (query, diags) = parse_all("RETURN;");
    assert!(query.is_none());
    assert!(!diags.is_empty());
    assert_eq!(diags.len(), 1);
}

#[test]
fn test_parse_with_label() {
    use open_cypher::parse_with_label;
    let result = parse_with_label("RETURN 1", "test.cypher");
    assert!(result.is_ok());

    let result = parse_with_label("RETURN;", "test.cypher");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.source_label(), Some("test.cypher"));
}
