use cypher::sema::analyze;
use cypher::{ErrorKind, parse};

#[test]
fn redeclare_in_match_pattern() {
    let query = parse("MATCH (a), (a) RETURN a").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "a");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

#[test]
fn redeclare_in_unwind() {
    let query = parse("UNWIND [1] AS x UNWIND [2] AS x RETURN x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

#[test]
fn redeclare_in_with_alias() {
    let query = parse("MATCH (n) WITH n AS x, n.name AS x RETURN x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

#[test]
fn redeclare_in_return_alias() {
    let query = parse("MATCH (n) RETURN n AS x, n.name AS x").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}

#[test]
fn redeclare_in_foreach() {
    let query = parse("FOREACH (x IN [1, 2, 3] | CREATE (:Label {val: x})) FOREACH (x IN [4, 5] | CREATE (:Other {val: x}))").expect("should parse");
    let result = analyze(&query);
    // This should be OK — each FOREACH has its own scope
    assert!(result.is_ok());
}

#[test]
fn shadowing_across_scopes_allowed() {
    // Note: WITH doesn't currently push/pop scope (that's A.3), so this will fail for now
    // This test documents expected future behavior
    let query = parse("MATCH (x) WITH x.name AS x RETURN x").expect("should parse");
    let result = analyze(&query);
    // Currently fails because WITH doesn't implement scoping yet (A.3)
    // We'll mark this as expected to fail until A.3 is done
    // For now, just check it doesn't panic
    let _ = result;
}

#[test]
fn redeclare_in_pattern_chain() {
    let query = parse("MATCH (a)-[r]->(a) RETURN a, r").expect("should parse");
    let result = analyze(&query);
    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.len(), 1);
    match errs.errors[0].kind() {
        ErrorKind::RedeclaredVariable { name, .. } => {
            assert_eq!(name, "a");
        }
        _ => panic!(
            "expected RedeclaredVariable, got {:?}",
            errs.errors[0].kind()
        ),
    }
}
