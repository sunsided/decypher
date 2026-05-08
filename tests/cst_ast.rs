//! Integration tests for the rowan-based typed CST (`decypher::cst`).
//!
//! These tests verify that CST nodes can be cast to the expected typed
//! wrappers and that specific fields and child nodes are accessible.

use assert2::check;
use decypher::cst::{AstNode, BinOp, BinaryExpr, Expression, UnOp, UnaryExpr, parse};

/// Every sample query parses and can be cast to a `SourceFile` CST node.
///
/// Unit: `decypher::cst::SourceFile::cast`
/// Precondition: A set of common Cypher queries.
/// Expectation: All queries produce a `Some(SourceFile)` on cast.
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
        let source = decypher::cst::SourceFile::cast(result.tree.clone());
        check!(source.is_some(), "Failed to cast for: {input}");
    }
}

/// Walk to the first `MATCH` clause in the first statement of `source`.
fn find_match_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::MatchClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Match(m) = clause {
            return Some(m);
        }
    }
    None
}

/// Walk to the first `RETURN` clause in the first statement of `source`.
fn find_return_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::ReturnClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Return(r) = clause {
            return Some(r);
        }
    }
    None
}

/// Walk to the first `DELETE` clause in the first statement of `source`.
fn find_delete_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::DeleteClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Delete(d) = clause {
            return Some(d);
        }
    }
    None
}

/// Walk to the first `SET` clause in the first statement of `source`.
fn find_set_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::SetClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Set(s) = clause {
            return Some(s);
        }
    }
    None
}

/// Walk to the first `UNWIND` clause in the first statement of `source`.
fn find_unwind_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::UnwindClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Unwind(u) = clause {
            return Some(u);
        }
    }
    None
}

/// Walk to the first `CREATE` clause in the first statement of `source`.
fn find_create_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::CreateClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Create(c) = clause {
            return Some(c);
        }
    }
    None
}

/// Walk to the first `MERGE` clause in the first statement of `source`.
fn find_merge_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::MergeClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Merge(m) = clause {
            return Some(m);
        }
    }
    None
}

/// Walk to the first `REMOVE` clause in the first statement of `source`.
fn find_remove_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::RemoveClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Remove(r) = clause {
            return Some(r);
        }
    }
    None
}

/// Verify CST shape for: match clause pattern parts.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: match clause with label.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: return clause projection.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn return_clause_projection() {
    let result = parse("RETURN 1 + 2");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();

    let proj = ret.projection_body().unwrap();
    let items: Vec<_> = proj.items().collect();
    check!(items.len() == 1);
}

/// Verify CST shape for: binary expr add.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: binary expr mul precedence.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: unary expr not.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: comparison expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: roundtrip text.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: can cast non matching returns none.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn can_cast_non_matching_returns_none() {
    use decypher::syntax::{CypherLang, SyntaxNode};
    use rowan::{GreenNodeBuilder, Language};

    let mut builder = GreenNodeBuilder::new();
    builder.start_node(CypherLang::kind_to_raw(
        decypher::syntax::SyntaxKind::SOURCE_FILE,
    ));
    builder.finish_node();
    let green = builder.finish();
    let node: SyntaxNode = rowan::SyntaxNode::new_root(green);

    check!(BinaryExpr::can_cast(node.kind()) == false);
    check!(decypher::cst::MatchClause::can_cast(node.kind()) == false);
    check!(decypher::cst::ReturnClause::can_cast(node.kind()) == false);

    check!(BinaryExpr::can_cast(decypher::syntax::SyntaxKind::OR_EXPR) == true);
    check!(BinaryExpr::can_cast(decypher::syntax::SyntaxKind::ADD_SUB_EXPR) == true);
    check!(UnaryExpr::can_cast(decypher::syntax::SyntaxKind::NOT_EXPR) == true);
    check!(decypher::cst::SourceFile::can_cast(decypher::syntax::SyntaxKind::SOURCE_FILE) == true);
}

/// Verify CST shape for: delete clause with detach.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn delete_clause_with_detach() {
    let result = parse("MATCH (n) DETACH DELETE n");
    let source = result.tree();
    let delete = find_delete_clause(&source).unwrap();
    check!(delete.detach_token().is_some());
    let exprs: Vec<_> = delete.exprs().collect();
    check!(exprs.len() == 1);
}

/// Verify CST shape for: set clause items.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: where clause in match.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn where_clause_in_match() {
    let result = parse("MATCH (n) WHERE n.age > 18 RETURN n");
    let source = result.tree();
    let match_clause = find_match_clause(&source).unwrap();
    check!(match_clause.where_clause().is_some());
}

/// Verify CST shape for: function invocation.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn function_invocation() {
    let result = parse("RETURN count(n)");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    match expr {
        Expression::Atom(decypher::cst::Atom::FunctionInvocation(fi)) => {
            check!(fi.name().is_some());
        }
        other => panic!("expected FunctionInvocation, got {other:?}"),
    }
}

/// Verify CST shape for: list literal elements.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn list_literal_elements() {
    let result = parse("RETURN [1, 2, 3]");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    match expr {
        Expression::Atom(decypher::cst::Atom::ListLiteral(list)) => {
            let elements: Vec<_> = list.elements().collect();
            check!(elements.len() == 3);
        }
        other => panic!("expected ListLiteral, got {other:?}"),
    }
}

/// Verify CST shape for: map literal entries.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn map_literal_entries() {
    let result = parse("RETURN {key: 'value'}");
    let source = result.tree();
    let ret = find_return_clause(&source).unwrap();
    let proj = ret.projection_body().unwrap();
    let item = proj.items().next().unwrap();
    let expr = item.expr().unwrap();
    match expr {
        Expression::Atom(decypher::cst::Atom::MapLiteral(map)) => {
            let entries: Vec<_> = map.entries().collect();
            check!(entries.len() == 1);
        }
        other => panic!("expected MapLiteral, got {other:?}"),
    }
}

/// Verify CST shape for: starts with expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: is null expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: is not null expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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

/// Verify CST shape for: parenthesized expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
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
        Expression::Atom(decypher::cst::Atom::Parenthesized(p)) => {
            check!(p.expr().is_some());
        }
        other => panic!("expected ParenthesizedExpr, got {other:?}"),
    }
}

/// Verify CST shape for: unwind clause.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn unwind_clause() {
    let result = parse("UNWIND [1, 2, 3] AS x RETURN x");
    let source = result.tree();
    let unwind = find_unwind_clause(&source).unwrap();
    check!(unwind.expr().is_some());
    check!(unwind.as_name().is_some());
}

/// Verify CST shape for: create clause.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn create_clause() {
    let result = parse("CREATE (n:Person {name: 'Alice'})");
    let source = result.tree();
    let create = find_create_clause(&source).unwrap();
    check!(create.pattern().is_some());
}

/// Verify CST shape for: merge clause.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn merge_clause() {
    let result = parse("MERGE (n:Person {name: 'Alice'}) ON CREATE SET n.created = timestamp()");
    let source = result.tree();
    let merge = find_merge_clause(&source).unwrap();
    check!(merge.pattern().is_some());
    let actions: Vec<_> = merge.actions().collect();
    check!(actions.len() >= 1);
}

/// Verify CST shape for: remove clause.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn remove_clause() {
    let result = parse("MATCH (n) REMOVE n.name");
    let source = result.tree();
    let remove = find_remove_clause(&source).unwrap();
    let items: Vec<_> = remove.items().collect();
    check!(items.len() == 1);
}

// ── New Phase B wrapper tests ────────────────────────────────────

fn find_foreach_clause(source: &decypher::cst::SourceFile) -> Option<decypher::cst::ForeachClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::Foreach(f) = clause {
            return Some(f);
        }
    }
    None
}

fn find_standalone_call(
    source: &decypher::cst::SourceFile,
) -> Option<decypher::cst::StandaloneCall> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::StandaloneCall(c) = clause {
            return Some(c);
        }
    }
    None
}

fn find_in_query_call(source: &decypher::cst::SourceFile) -> Option<decypher::cst::InQueryCall> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::InQueryCall(c) = clause {
            return Some(c);
        }
    }
    None
}

fn find_call_subquery(
    source: &decypher::cst::SourceFile,
) -> Option<decypher::cst::CallSubqueryClause> {
    for clause in source.statements().next()?.clauses() {
        if let decypher::cst::Clause::CallSubquery(c) = clause {
            return Some(c);
        }
    }
    None
}

/// Verify CST shape for: foreach clause.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn foreach_clause() {
    let result = parse("MATCH (n) FOREACH (x IN [1,2,3] | SET n.val = x) RETURN n");
    let source = result.tree();
    let foreach = find_foreach_clause(&source).unwrap();
    check!(foreach.variable().is_some());
    check!(foreach.list().is_some());
    let clauses: Vec<_> = foreach.clauses().collect();
    check!(clauses.len() >= 1);
}

/// Verify CST shape for: debug cst dump.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn debug_cst_dump() {
    let result = parse("MATCH (n) WHERE EXISTS { (n)-->(m) } RETURN n");
    let source = result.tree();
    fn dump(node: &decypher::syntax::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text = node.text().to_string();
        let text_preview = if text.len() > 60 {
            format!("{}...", &text[..57])
        } else {
            text
        };
        if node.children().next().is_none() {
            println!("{}{:?}: {:?}", prefix, node.kind(), text_preview);
        } else {
            println!("{}{:?}", prefix, node.kind());
            for child in node.children() {
                dump(&child, indent + 1);
            }
        }
    }
    dump(source.syntax(), 0);
}

/// Verify CST shape for: relationships pattern can cast.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn relationships_pattern_can_cast() {
    use decypher::syntax::SyntaxKind;
    check!(decypher::cst::RelationshipsPattern::can_cast(
        SyntaxKind::RELATIONSHIPS_PATTERN
    ));
    check!(!decypher::cst::RelationshipsPattern::can_cast(
        SyntaxKind::PATTERN
    ));
    check!(!decypher::cst::RelationshipsPattern::can_cast(
        SyntaxKind::NODE_PATTERN
    ));
}

/// Verify CST shape for: yield items star.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn yield_items_star() {
    let result = parse("MATCH (n) CALL apoc.load.json('url') YIELD value RETURN value");
    let source = result.tree();
    let yield_items = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::YieldItems::cast);
    let yield_items = yield_items.expect("YieldItems found");
    let items: Vec<_> = yield_items.items().collect();
    check!(!items.is_empty());
}

/// Verify CST shape for: yield item with alias.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn yield_item_with_alias() {
    let result = parse("MATCH (n) CALL apoc.load.json('url') YIELD value AS v RETURN v");
    let source = result.tree();
    let item = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::YieldItem::cast);
    check!(item.is_some(), "YieldItem found");
    let item = item.unwrap();
    check!(item.field_name().is_some());
}

/// Verify CST shape for: standalone call with yield.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn standalone_call_with_yield() {
    let result = parse("MATCH (n) CALL db.labels() YIELD label RETURN label");
    let source = result.tree();
    let call = find_standalone_call(&source).unwrap();
    check!(call.implicit_invocation().is_some());
}

/// Verify CST shape for: implicit procedure invocation.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn implicit_procedure_invocation() {
    let result = parse("CALL db.labels");
    let source = result.tree();
    let call = find_standalone_call(&source).unwrap();
    check!(call.implicit_invocation().is_some());
    let implicit = call.implicit_invocation().unwrap();
    check!(implicit.procedure_name().is_some());
}

/// Verify CST shape for: in query call.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn in_query_call() {
    let result = parse("MATCH (n) YIELD foo RETURN n");
    let source = result.tree();
    let in_query = find_in_query_call(&source).unwrap();
    check!(in_query.yield_items().is_some());
}

/// Verify CST shape for: call subquery with in transactions.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn call_subquery_with_in_transactions() {
    let result = parse("MATCH (n) CALL { RETURN n } IN TRANSACTIONS OF 1000 ROWS");
    let source = result.tree();
    let call_subquery = find_call_subquery(&source).unwrap();
    check!(call_subquery.in_transactions().is_some());
    let in_tx = call_subquery.in_transactions().unwrap();
    check!(in_tx.rows_expr().is_some());
}

/// Verify CST shape for: call subquery with on error continue.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn call_subquery_with_on_error_continue() {
    let result = parse("MATCH (n) CALL { RETURN n } IN TRANSACTIONS ON ERROR CONTINUE");
    let source = result.tree();
    let call_subquery = find_call_subquery(&source).unwrap();
    let in_tx = call_subquery.in_transactions().unwrap();
    check!(in_tx.on_error_action().is_some());
    let action = in_tx.on_error_action().unwrap();
    check!(action.text() == "CONTINUE");
}

/// Verify CST shape for: show clause with kind.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn show_clause_with_kind() {
    let result = parse("SHOW INDEXES");
    let source = result.tree();
    let show = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::ShowClause::cast)
        .unwrap();
    check!(show.kind().is_some());
}

/// Verify CST shape for: use clause.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn use_clause() {
    let result = parse("USE mydb");
    let source = result.tree();
    let use_clause = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::UseClause::cast)
        .unwrap();
    check!(use_clause.schema_name().is_some());
    let schema_name = use_clause.schema_name().unwrap();
    check!(schema_name.symbolic_name().is_some());
}

/// Verify CST shape for: union all.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn union_all() {
    let result = parse("MATCH (n:Person) RETURN n.name UNION ALL MATCH (m:Company) RETURN m.name");
    let source = result.tree();
    let union = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::Union::cast);
    check!(union.is_some(), "Union found");
    let union = union.unwrap();
    check!(union.all_token().is_some());
}

/// Verify CST shape for: create index.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn create_index() {
    let result = parse("CREATE INDEX my_index FOR (n:Person) ON (n.name)");
    let source = result.tree();
    let create_index = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::CreateIndex::cast);
    check!(create_index.is_some(), "CreateIndex found");
    let ci = create_index.unwrap();
    check!(ci.if_not_exists() == false);
    check!(ci.name().is_some());
}

/// Verify CST shape for: create index if not exists.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn create_index_if_not_exists() {
    let result = parse("CREATE INDEX IF NOT EXISTS my_index FOR (n:Person) ON (n.name)");
    let source = result.tree();
    let ci = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::CreateIndex::cast)
        .unwrap();
    check!(ci.if_not_exists() == true);
}

/// Verify CST shape for: drop index.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn drop_index() {
    let result = parse("DROP INDEX my_index");
    let source = result.tree();
    let drop_idx = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::DropIndex::cast);
    check!(drop_idx.is_some(), "DropIndex found");
    let di = drop_idx.unwrap();
    check!(di.name().is_some());
    check!(di.if_exists() == false);
}

/// Verify CST shape for: drop index if exists.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn drop_index_if_exists() {
    let result = parse("DROP INDEX my_index IF EXISTS");
    let source = result.tree();
    let di = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::DropIndex::cast)
        .unwrap();
    check!(di.if_exists() == true);
}

/// Verify CST shape for: create constraint.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn create_constraint() {
    let result = parse("CREATE CONSTRAINT my_constraint FOR (n:Person) REQUIRE n.name IS UNIQUE");
    let source = result.tree();
    let cc = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::CreateConstraint::cast);
    check!(cc.is_some(), "CreateConstraint found");
}

/// Verify CST shape for: drop constraint.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn drop_constraint() {
    let result = parse("DROP CONSTRAINT my_constraint IF EXISTS");
    let source = result.tree();
    let dc = source
        .syntax()
        .descendants()
        .find_map(decypher::cst::DropConstraint::cast);
    check!(dc.is_some(), "DropConstraint found");
    let dc = dc.unwrap();
    check!(dc.if_exists() == true);
}

/// Verify CST shape for: debug cst dump rel.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn debug_cst_dump_rel() {
    let result = parse("MATCH (a)-[r]->(b) RETURN a, r, b");
    let source = result.tree();
    fn dump(node: &decypher::syntax::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text = node.text().to_string();
        let text_preview = if text.len() > 60 {
            format!("{}...", &text[..57])
        } else {
            text
        };
        if node.children().next().is_none() {
            println!("{}{:?}: {:?}", prefix, node.kind(), text_preview);
        } else {
            println!("{}{:?}", prefix, node.kind());
            for child in node.children() {
                dump(&child, indent + 1);
            }
        }
    }
    dump(source.syntax(), 0);
}

#[test]
fn debug_cst_dump_rel2() {
    let result = parse("MATCH (a)-[r]->(b) RETURN a, r, b");
    let source = result.tree();
    for chain in source
        .syntax()
        .descendants()
        .filter_map(decypher::cst::PatternElementChain::cast)
    {
        println!(
            "PatternElementChain syntax kind: {:?}",
            chain.syntax().kind()
        );
        for child in chain.syntax().children() {
            println!("  child: {:?}", child.kind());
        }
        for token in chain
            .syntax()
            .children_with_tokens()
            .filter_map(|t| t.into_token())
        {
            println!("  token: {:?} = {:?}", token.kind(), token.text());
        }
        println!("  relationship(): {:?}", chain.relationship());
        let detail = chain
            .syntax()
            .children()
            .find_map(decypher::cst::RelationshipDetail::cast);
        println!(
            "  detail (direct): {:?}",
            detail.map(|d| d.syntax().text().to_string())
        );
    }
}
/// Verify CST shape for: debug cst dump case.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn debug_cst_dump_case() {
    let result = parse("MATCH (n) RETURN CASE WHEN n.active THEN 'yes' ELSE 'no' END");
    let source = result.tree();
    fn dump(node: &decypher::syntax::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text = node.text().to_string();
        let text_preview = if text.len() > 60 {
            format!("{}...", &text[..57])
        } else {
            text
        };
        if node.children().next().is_none() {
            println!("{}{:?}: {:?}", prefix, node.kind(), text_preview);
        } else {
            println!("{}{:?}", prefix, node.kind());
            for child in node.children() {
                dump(&child, indent + 1);
            }
        }
    }
    dump(source.syntax(), 0);
}

/// Verify CST shape for: debug cst dump is not null.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn debug_cst_dump_is_not_null() {
    let result = parse("RETURN x IS NOT NULL");
    let source = result.tree();
    fn dump(node: &decypher::syntax::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text = node.text().to_string();
        let text_preview = if text.len() > 60 {
            format!("{}...", &text[..57])
        } else {
            text
        };
        if node.children().next().is_none() {
            println!("{}{:?}: {:?}", prefix, node.kind(), text_preview);
        } else {
            println!("{}{:?}", prefix, node.kind());
            for child in node.children() {
                dump(&child, indent + 1);
            }
        }
    }
    dump(source.syntax(), 0);

    // Also check the expression type
    for proj in source
        .syntax()
        .descendants()
        .filter_map(decypher::cst::ProjectionItem::cast)
    {
        if let Some(expr) = proj.expr() {
            println!("Expression: {:?}", expr);
            match expr {
                decypher::cst::Expression::BinaryExpr(b) => {
                    println!("  BinaryExpr op_kind: {:?}", b.op_kind());
                }
                other => {
                    println!("  Other: {:?}", other);
                }
            }
        }
    }
}

/// Verify CST shape for: debug cst dump null.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn debug_cst_dump_null() {
    let result = parse("RETURN null;");
    let source = result.tree();
    fn dump(node: &decypher::syntax::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text = node.text().to_string();
        let text_preview = if text.len() > 60 {
            format!("{}...", &text[..57])
        } else {
            text
        };
        if node.children().next().is_none() {
            println!("{}{:?}: {:?}", prefix, node.kind(), text_preview);
        } else {
            println!("{}{:?}", prefix, node.kind());
            for child in node.children() {
                dump(&child, indent + 1);
            }
        }
    }
    dump(source.syntax(), 0);

    // Check what ProjectionItem sees
    for proj in source
        .syntax()
        .descendants()
        .filter_map(decypher::cst::ProjectionItem::cast)
    {
        println!("ProjectionItem children:");
        for child in proj.syntax().children() {
            println!("  node: {:?}", child.kind());
        }
        for t in proj
            .syntax()
            .children_with_tokens()
            .filter_map(|t| t.into_token())
        {
            println!("  token: {:?} = {:?}", t.kind(), t.text());
        }
        println!("  expr(): {:?}", proj.expr());
    }
}

/// Verify CST shape for: constraint composite node key.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn constraint_composite_node_key() {
    use decypher::cst::parse as cst_parse;
    let result = cst_parse(
        "CREATE CONSTRAINT composite_key FOR (p:Person) REQUIRE (p.country, p.id) IS NODE KEY;",
    );
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let cmd = source
        .schema_commands()
        .next()
        .expect("expected schema command");
    match cmd {
        decypher::cst::SchemaCommand::CreateConstraint(cc) => {
            let ck = cc.constraint_kind().expect("expected constraint kind");
            let tokens: Vec<_> = ck
                .syntax()
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .collect();
            check!(
                tokens.iter().any(|t| t.to_string() == "NODE"),
                "expected NODE token in constraint kind"
            );
            check!(
                tokens.iter().any(|t| t.to_string() == "KEY"),
                "expected KEY token in constraint kind"
            );
        }
        other => panic!("expected CreateConstraint, got {:?}", other),
    }
}

/// Verify CST shape for: index label alternatives.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn index_label_alternatives() {
    use decypher::cst::parse as cst_parse;
    let result =
        cst_parse("CREATE FULLTEXT INDEX person_names FOR (p:Person|Employee) ON EACH [p.name];");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let cmd = source
        .schema_commands()
        .next()
        .expect("expected schema command");
    match cmd {
        decypher::cst::SchemaCommand::CreateIndex(ci) => {
            let labels: Vec<_> = ci
                .syntax()
                .descendants()
                .filter(|n| n.kind() == decypher::syntax::SyntaxKind::NODE_LABEL)
                .collect();
            check!(labels.len() == 2, "expected 2 labels, got {}", labels.len());
        }
        other => panic!("expected CreateIndex, got {:?}", other),
    }
}

/// Verify CST shape for: index relationship type alternatives.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn index_relationship_type_alternatives() {
    use decypher::cst::parse as cst_parse;
    let result =
        cst_parse("CREATE FULLTEXT INDEX rel_names FOR ()-[r:KNOWS|LIKES]-() ON EACH [r.since];");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let cmd = source
        .schema_commands()
        .next()
        .expect("expected schema command");
    match cmd {
        decypher::cst::SchemaCommand::CreateIndex(ci) => {
            let rel_type_names: Vec<_> = ci
                .syntax()
                .descendants()
                .filter(|n| n.kind() == decypher::syntax::SyntaxKind::REL_TYPE_NAME)
                .collect();
            check!(
                rel_type_names.len() == 2,
                "expected 2 rel type names, got {}",
                rel_type_names.len()
            );
        }
        other => panic!("expected CreateIndex, got {:?}", other),
    }
}

/// Verify CST shape for: collect subquery expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn collect_subquery_expr() {
    use decypher::cst::parse as cst_parse;
    let result = cst_parse("RETURN COLLECT { MATCH (n) RETURN n };");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let collect_subqueries: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::COLLECT_SUBQUERY)
        .collect();
    check!(
        collect_subqueries.len() == 1,
        "expected 1 COLLECT_SUBQUERY node, got {}",
        collect_subqueries.len()
    );
}

/// Verify CST shape for: count subquery expr.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn count_subquery_expr() {
    use decypher::cst::parse as cst_parse;
    let result = cst_parse("RETURN COUNT { MATCH (n) RETURN n };");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let count_subqueries: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::COUNT_SUBQUERY)
        .collect();
    check!(
        count_subqueries.len() == 1,
        "expected 1 COUNT_SUBQUERY node, got {}",
        count_subqueries.len()
    );
}

/// Verify CST shape for: label expression cst nodes.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn label_expression_cst_nodes() {
    use decypher::cst::parse as cst_parse;
    let result = cst_parse("MATCH (n:(Person|Company)&!Deleted) RETURN n;");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let label_exprs: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::LABEL_EXPRESSION)
        .collect();
    let and_nodes: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::LABEL_AND)
        .collect();
    let or_nodes: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::LABEL_OR)
        .collect();
    let not_nodes: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::LABEL_NOT)
        .collect();
    check!(label_exprs.len() == 1);
    check!(and_nodes.len() >= 1);
    check!(or_nodes.len() >= 1);
    check!(not_nodes.len() >= 1);
}

/// Verify CST shape for: quantified path pattern cst node.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn quantified_path_pattern_cst_node() {
    use decypher::cst::parse as cst_parse;
    let result = cst_parse("MATCH p = ((a)-[:R]->(b)){1,3} RETURN p;");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let quantified: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::QUANTIFIED_PATH_PATTERN)
        .collect();
    check!(
        quantified.len() == 1,
        "expected 1 quantified path node, got {}",
        quantified.len()
    );
}

/// Verify CST shape for: dynamic label cst node.
///
/// Unit: rowan CST typed wrappers
/// Precondition: Valid Cypher input.
/// Expectation: Expected CST node types are present.
#[test]
fn dynamic_label_cst_node() {
    use decypher::cst::parse as cst_parse;
    let result = cst_parse("MATCH (n:$(label)) RETURN n;");
    check!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    let source = result.tree();
    let dynamic: Vec<_> = source
        .syntax()
        .descendants()
        .filter(|n| n.kind() == decypher::syntax::SyntaxKind::DYNAMIC_LABEL)
        .collect();
    check!(
        dynamic.len() == 1,
        "expected 1 dynamic label node, got {}",
        dynamic.len()
    );
}
