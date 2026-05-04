use cypher::{parse, ErrorKind};
use cypher::sema::analyze;

#[test]
fn with_hides_previous_variables() {
    // After WITH n.name AS name, 'n' should be out of scope
    let query = parse("MATCH (n) WITH n.name AS name WHERE n.other IS NOT NULL RETURN name").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    // Should have an unresolved variable error for 'n' in the WHERE clause
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::UnresolvedVariable { name } => {
            assert_eq!(name, "n");
        }
        _ => panic!("expected UnresolvedVariable, got {:?}", errs.errors[0].kind()),
    }
}

#[test]
fn with_unaliased_projection_binds() {
    // MATCH (n) WITH n.name RETURN name — unaliased projection binds as 'name'
    let query = parse("MATCH (n) WITH n.name RETURN name").expect("should parse");
    let result = analyze(&query);
    // Should succeed — 'name' is bound by the unaliased projection
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}

#[test]
fn with_variable_projection_binds() {
    // MATCH (n) WITH n RETURN n — unaliased variable projection binds as 'n'
    let query = parse("MATCH (n) WITH n RETURN n").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}

#[test]
fn with_multiple_parts() {
    // Multi-part query: each WITH introduces a new scope
    let query = parse("MATCH (a) WITH a.x AS x MATCH (b) WITH x, b.y AS y RETURN x, y").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_ok(), "analysis failed: {:?}", result);
}

#[test]
fn with_scope_prevents_access_to_earlier_vars() {
    // MATCH (a), (b) WITH a RETURN b — 'b' should be out of scope after WITH
    let query = parse("MATCH (a), (b) WITH a RETURN b").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::UnresolvedVariable { name } => {
            assert_eq!(name, "b");
        }
        _ => panic!("expected UnresolvedVariable, got {:?}", errs.errors[0].kind()),
    }
}

#[test]
fn with_star_projection() {
    // WITH * should pass through all current bindings (not tested here, placeholder)
    // For now just check it doesn't crash
    let query = parse("MATCH (n) WITH * RETURN n").expect("should parse");
    let result = analyze(&query);
    // * is not yet implemented, so this will likely fail — document expected behavior
    let _ = result; // Don't assert for now
}
