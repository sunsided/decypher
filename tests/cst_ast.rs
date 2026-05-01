use assert2::check;
use open_cypher::cst::{parse, AstNode, BinOp, BinaryExpr, Expression, UnOp, UnaryExpr};

/// Every sample query from tests/smoke.rs parses and casts to SourceFile.
#[test]
fn smoke_queries_cast_to_source_file() {
    let queries = [
        "MATCH (n) RETURN n",
        "MATCH (n:Person) RETURN n.name",
        "MATCH (n) RETURN n",
        "MATCH (a)-[r]->(b) RETURN a, r, b",
        "MATCH (n) WHERE n.age > 18 RETURN n",
        "RETURN 1 + 2 * 3",
        "RETURN NOT true",
        "RETURN name STARTS WITH 'A'",
        "RETURN x IS NOT NULL",
        "RETURN (1 + 2) * 3",
        "MATCH (n:Person) RETURN n.name",
        "RETURN a.b.c.d",
        "RETURN count(n)",
        "RETURN [1, 2, 3]",
        "RETURN {key: 'value'}",
    ];
    for input in queries {
        let result = parse(input);
        let source = open_cypher::cst::SourceFile::cast(result.tree.clone());
        assert!(source.is_some(), "Failed to cast for: {input}");
    }
}

fn find_match_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::MatchClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Match(m) = clause {
            return Some(m);
        }
    }
    None
}

fn find_return_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::ReturnClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Return(r) = clause {
            return Some(r);
        }
    }
    None
}

fn find_delete_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::DeleteClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Delete(d) = clause {
            return Some(d);
        }
    }
    None
}

fn find_set_clause(source: &open_cypher::cst::SourceFile) -> Option<open_cypher::cst::SetClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Set(s) = clause {
            return Some(s);
        }
    }
    None
}

fn find_unwind_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::UnwindClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Unwind(u) = clause {
            return Some(u);
        }
    }
    None
}

fn find_create_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::CreateClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Create(c) = clause {
            return Some(c);
        }
    }
    None
}

fn find_merge_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::MergeClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Merge(m) = clause {
            return Some(m);
        }
    }
    None
}

fn find_remove_clause(
    source: &open_cypher::cst::SourceFile,
) -> Option<open_cypher::cst::RemoveClause> {
    for clause in source.statements().next()?.clauses() {
        if let open_cypher::cst::Clause::Remove(r) = clause {
            return Some(r);
        }
    }
    None
}

#[test]
fn match_clause_pattern_parts() {
    let result = parse("MATCH (n:Person) RETURN n.name");
    let source = result.tree();
    let stmt = source.statements().next().unwrap();
    let clauses: Vec<_> = stmt.clauses().collect();
    check!(clauses.len() == 2);

    let match_clause = find_match_clause(&source).unwrap();
    check!(match_clause.optional_token().is_none());
    let pattern = match_clause.pattern().unwrap();
    let parts: Vec<_> = pattern.parts().collect();
    check!(parts.len() == 1);
}

#[test]
fn match_clause_with_label() {
    let result = parse("MATCH (n:Person) RETURN n.name");
    let source = result.tree();
    let match_clause = find_match_clause(&source).unwrap();

    let pattern = match_clause.pattern().unwrap();
    let part = pattern.parts().next().unwrap();
    let element = part.element().unwrap();
    let node = element.node().unwrap();
    let labels: Vec<_> = node.labels().collect();
    check!(labels.len() == 1);
}

#[test]
fn return_clause_projection() {
    let result = parse("RETURN 1 + 2");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();

    let proj = ret.projection_body().unwrap();
    let items: Vec<_> = proj.items().collect();
    check!(items.len() == 1);
}

#[test]
fn binary_expr_add() {
    let result = parse("RETURN 1 + 2");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let bin = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr, got {expr:?}"),
    };
    check!(bin.op_kind() == Some(BinOp::Add));
}

#[test]
fn binary_expr_mul_precedence() {
    let result = parse("RETURN 1 + 2 * 3");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();

    let add = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr at top"),
    };
    check!(add.op_kind() == Some(BinOp::Add));

    let rhs = add.rhs().unwrap();
    let mul = match rhs {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr on rhs"),
    };
    check!(mul.op_kind() == Some(BinOp::Mul));
}

#[test]
fn unary_expr_not() {
    let result = parse("RETURN NOT true");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let unary = match expr {
        Expression::UnaryExpr(u) => u,
        _ => panic!("expected UnaryExpr"),
    };
    check!(unary.op() == Some(UnOp::Not));
}

#[test]
fn comparison_expr() {
    let result = parse("RETURN n.age > 18");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let bin = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr"),
    };
    check!(bin.op_kind() == Some(BinOp::Gt));
}

#[test]
fn roundtrip_text() {
    let inputs = [
        "MATCH (n) RETURN n",
        "MATCH (n:Person) RETURN n.name",
        "RETURN 1 + 2 * 3",
        "RETURN NOT true",
        "RETURN name STARTS WITH 'A'",
        "RETURN x IS NOT NULL",
        "RETURN (1 + 2) * 3",
    ];
    for input in inputs {
        let result = parse(input);
        let source = result.tree();
        check!(source.syntax().text().to_string() == input);
    }
}

#[test]
fn can_cast_non_matching_returns_none() {
    use open_cypher::syntax::{CypherLang, SyntaxNode};
    use rowan::{GreenNodeBuilder, Language};

    let mut builder = GreenNodeBuilder::new();
    builder.start_node(CypherLang::kind_to_raw(
        open_cypher::syntax::SyntaxKind::SOURCE_FILE,
    ));
    builder.finish_node();
    let green = builder.finish();
    let node: SyntaxNode = rowan::SyntaxNode::new_root(green);

    check!(BinaryExpr::can_cast(node.kind()) == false);
    check!(open_cypher::cst::MatchClause::can_cast(node.kind()) == false);
    check!(open_cypher::cst::ReturnClause::can_cast(node.kind()) == false);

    check!(BinaryExpr::can_cast(open_cypher::syntax::SyntaxKind::OR_EXPR) == true);
    check!(BinaryExpr::can_cast(open_cypher::syntax::SyntaxKind::ADD_SUB_EXPR) == true);
    check!(UnaryExpr::can_cast(open_cypher::syntax::SyntaxKind::NOT_EXPR) == true);
    check!(
        open_cypher::cst::SourceFile::can_cast(open_cypher::syntax::SyntaxKind::SOURCE_FILE)
            == true
    );
}

#[test]
fn delete_clause_with_detach() {
    let result = parse("MATCH (n) DETACH DELETE n");
    let source = result.tree();
    let delete = find_delete_clause(&source).unwrap();
    check!(delete.detach_token().is_some());
    let exprs: Vec<_> = delete.exprs().collect();
    check!(exprs.len() == 1);
}

#[test]
fn set_clause_items() {
    let result = parse("MATCH (n) SET n.name = 'Alice'");
    let source = result.tree();
    let set = find_set_clause(&source).unwrap();
    let items: Vec<_> = set.items().collect();
    check!(items.len() == 1);
    let item = &items[0];
    check!(item.eq_token().is_some());
}

#[test]
fn where_clause_in_match() {
    let result = parse("MATCH (n) WHERE n.age > 18 RETURN n");
    let source = result.tree();
    let match_clause = find_match_clause(&source).unwrap();
    check!(match_clause.where_clause().is_some());
}

#[test]
fn function_invocation() {
    let result = parse("RETURN count(n)");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    match expr {
        Expression::Atom(open_cypher::cst::Atom::FunctionInvocation(fi)) => {
            check!(fi.name().is_some());
        }
        other => panic!("expected FunctionInvocation, got {other:?}"),
    }
}

#[test]
fn list_literal_elements() {
    let result = parse("RETURN [1, 2, 3]");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    match expr {
        Expression::Atom(open_cypher::cst::Atom::ListLiteral(list)) => {
            let elements: Vec<_> = list.elements().collect();
            check!(elements.len() == 3);
        }
        other => panic!("expected ListLiteral, got {other:?}"),
    }
}

#[test]
fn map_literal_entries() {
    let result = parse("RETURN {key: 'value'}");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    match expr {
        Expression::Atom(open_cypher::cst::Atom::MapLiteral(map)) => {
            let entries: Vec<_> = map.entries().collect();
            check!(entries.len() == 1);
        }
        other => panic!("expected MapLiteral, got {other:?}"),
    }
}

#[test]
fn starts_with_expr() {
    let result = parse("RETURN name STARTS WITH 'A'");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let bin = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr"),
    };
    check!(bin.op_kind() == Some(BinOp::StartsWith));
}

#[test]
fn is_null_expr() {
    let result = parse("RETURN x IS NULL");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let bin = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr"),
    };
    check!(bin.op_kind() == Some(BinOp::IsNull));
}

#[test]
fn is_not_null_expr() {
    let result = parse("RETURN x IS NOT NULL");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let bin = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr"),
    };
    check!(bin.op_kind() == Some(BinOp::IsNotNull));
}

#[test]
fn parenthesized_expr() {
    let result = parse("RETURN (1 + 2) * 3");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    let mul = match expr {
        Expression::BinaryExpr(b) => b,
        _ => panic!("expected BinaryExpr"),
    };
    check!(mul.op_kind() == Some(BinOp::Mul));
    let lhs = mul.lhs().unwrap();
    match lhs {
        Expression::Atom(open_cypher::cst::Atom::Parenthesized(p)) => {
            check!(p.expr().is_some());
        }
        other => panic!("expected ParenthesizedExpr, got {other:?}"),
    }
}

#[test]
fn unwind_clause() {
    let result = parse("UNWIND [1, 2, 3] AS x RETURN x");
    let source = result.tree();
    let unwind = find_unwind_clause(&source).unwrap();
    check!(unwind.expr().is_some());
    check!(unwind.as_name().is_some());
}

#[test]
fn create_clause() {
    let result = parse("CREATE (n:Person {name: 'Alice'})");
    let source = result.tree();
    let create = find_create_clause(&source).unwrap();
    check!(create.pattern().is_some());
}

#[test]
fn merge_clause() {
    let result = parse("MERGE (n:Person {name: 'Alice'}) ON CREATE SET n.created = timestamp()");
    let source = result.tree();
    let merge = find_merge_clause(&source).unwrap();
    check!(merge.pattern().is_some());
    let actions: Vec<_> = merge.actions().collect();
    check!(actions.len() >= 1);
}

#[test]
fn remove_clause() {
    let result = parse("MATCH (n) REMOVE n.name");
    let source = result.tree();
    let remove = find_remove_clause(&source).unwrap();
    let items: Vec<_> = remove.items().collect();
    check!(items.len() == 1);
}
