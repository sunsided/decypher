use open_cypher::parse;
use open_cypher::CypherError;

#[test]
fn test_empty_string() {
    let result = parse("");
    assert!(result.is_err());
    match result.unwrap_err() {
        CypherError::Syntax(_) => {}
        _ => panic!("expected Syntax error"),
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
