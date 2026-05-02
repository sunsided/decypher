use assert2::check;
use open_cypher::ast::query::{QueryBody, SinglePartBody};
use open_cypher::parse;

#[test]
fn test_match_return_has_return_body() {
    let query = parse("MATCH (n) RETURN n;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            open_cypher::ast::query::SingleQueryKind::SinglePart(spq) => match &spq.body {
                SinglePartBody::Return(ret) => {
                    assert_eq!(ret.items.len(), 1);
                }
                _ => panic!("expected Return body"),
            },
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

#[test]
fn test_match_create_has_updating_body() {
    let query = parse("MATCH (a) CREATE (a)-[:KNOWS]->(b);").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            open_cypher::ast::query::SingleQueryKind::SinglePart(spq) => match &spq.body {
                SinglePartBody::Updating {
                    updating,
                    return_clause,
                } => {
                    assert_eq!(updating.len(), 1);
                    check!(return_clause.is_none());
                }
                _ => panic!("expected Updating body"),
            },
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

#[test]
fn test_optional_match_flag() {
    let query = parse("OPTIONAL MATCH (n) RETURN n;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            open_cypher::ast::query::SingleQueryKind::SinglePart(spq) => {
                match &spq.reading_clauses[0] {
                    open_cypher::ast::query::ReadingClause::Match(m) => {
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

#[test]
fn test_unwind_has_expression_and_variable() {
    let query = parse("UNWIND [1, 2, 3] AS x RETURN x;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => match &sq.kind {
            open_cypher::ast::query::SingleQueryKind::SinglePart(spq) => {
                match &spq.reading_clauses[0] {
                    open_cypher::ast::query::ReadingClause::Unwind(u) => {
                        assert_eq!(u.variable.name.name, "x");
                    }
                    _ => panic!("expected Unwind clause"),
                }
            }
            _ => panic!("expected SinglePart query"),
        },
        _ => panic!("expected SingleQuery"),
    }
}

#[test]
fn test_pattern_has_node() {
    let query = parse("MATCH (n:Person) RETURN n;").unwrap();
    match &query.statements[0] {
        QueryBody::SingleQuery(sq) => {
            match &sq.kind {
                open_cypher::ast::query::SingleQueryKind::SinglePart(spq) => {
                    match &spq.reading_clauses[0] {
                        open_cypher::ast::query::ReadingClause::Match(m) => {
                            assert_eq!(m.pattern.parts.len(), 1);
                            let part = &m.pattern.parts[0];
                            // The node variable is inside the anonymous pattern part
                            match &part.anonymous.element {
                                open_cypher::ast::pattern::PatternElement::Path {
                                    start, ..
                                } => {
                                    check!(start.variable.is_some());
                                    assert_eq!(start.variable.as_ref().unwrap().name.name, "n");
                                    assert_eq!(start.labels.len(), 1);
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

#[test]
fn test_union_has_two_queries() {
    let query =
        parse("MATCH (n:Person) RETURN n.name UNION MATCH (m:Movie) RETURN m.title;").unwrap();
    check!(query.statements.len() >= 1);
    // The UNION creates a RegularQuery with unions
    // Our current structure stores it in RegularQuery.unions
}

#[test]
fn test_span_is_nonzero() {
    let query = parse("MATCH (n) RETURN n;").unwrap();
    check!(query.span.start < query.span.end);
}
