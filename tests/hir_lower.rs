//! Integration tests for the HIR lowering pass (`decypher::analyze`).
//!
//! These tests call [`decypher::analyze`] on Cypher strings and verify the
//! shape of the resulting [`decypher::hir::HirQuery`].

use assert2::check;
use decypher::analyze;
use decypher::hir::{
    ExprKind, LowerConfig, RelationshipDirection,
    expr::{BinaryOp, Literal as HirLiteral},
    ops::ProjectOp,
    ops::{AggregateOp, MatchOp, Operation},
};

fn find_match_operation(operations: &[Operation]) -> &MatchOp {
    operations
        .iter()
        .find_map(|op| {
            if let Operation::Match(m) = op {
                Some(m)
            } else {
                None
            }
        })
        .expect("expected a Match operation")
}

fn find_first_project_expression(hir: &decypher::hir::HirQuery) -> &ExprKind {
    let part = hir.parts.first().expect("expected at least one query part");
    let project = part
        .operations
        .iter()
        .find_map(|op| match op {
            Operation::Project(project) => Some(project),
            _ => None,
        })
        .expect("expected a Project operation");
    let expression_id = project
        .items
        .first()
        .expect("expected at least one projection item")
        .expression;
    &hir.arenas.expressions.get(expression_id).kind
}

fn find_project_operation(operations: &[Operation]) -> &ProjectOp {
    operations
        .iter()
        .find_map(|op| {
            if let Operation::Project(project) = op {
                Some(project)
            } else {
                None
            }
        })
        .expect("expected a Project operation")
}

fn assert_integer_literal(
    hir: &decypher::hir::HirQuery,
    expr_id: decypher::hir::ExprId,
    value: i64,
) {
    let expr = hir.arenas.expressions.get(expr_id);
    check!(
        matches!(expr.kind, ExprKind::Literal(HirLiteral::Integer(v)) if v == value),
        "expected integer literal {value}, got {:?}",
        expr.kind
    );
}

/// A basic `MATCH … RETURN` query lowers to exactly one query part.
///
/// Unit: `analyze()`
/// Precondition: A single-part query with a MATCH and a RETURN clause.
/// Expectation: `hir.parts.len() == 1`.
#[test]
fn analyze_basic_query() {
    let hir = analyze("MATCH (p:Person)-[:KNOWS]->(f) WHERE p.age > 18 RETURN f.name").unwrap();
    check!(hir.parts.len() == 1);
}

/// A `WITH`-split query lowers to two query parts.
///
/// Unit: `analyze()`
/// Precondition: Query has a MATCH → WITH → RETURN structure (two parts).
/// Expectation: `hir.parts.len() == 2`.
#[test]
fn analyze_multi_part() {
    let hir = analyze("MATCH (p:Person) WITH p, count(*) AS cnt WHERE cnt > 3 RETURN p.name, cnt")
        .unwrap();
    check!(hir.parts.len() == 2);
}

/// Referencing an unbound variable (`x`) produces an error.
///
/// Unit: `analyze()`
/// Precondition: `x` is never bound in the query.
/// Expectation: `analyze` returns `Err`.
#[test]
fn analyze_unknown_variable() {
    let result = analyze("MATCH (p:Person) RETURN x.name");
    check!(result.is_err());
}

/// A `CREATE` query with no RETURN lowers to one query part.
///
/// Unit: `analyze()`
/// Precondition: Single CREATE clause with inline property map.
/// Expectation: `hir.parts.len() == 1`.
#[test]
fn analyze_create_query() {
    let hir = analyze("CREATE (p:Person {name: 'Alice'})").unwrap();
    check!(hir.parts.len() == 1);
}

/// An `OPTIONAL MATCH … RETURN` query lowers to one query part.
///
/// Unit: `analyze()`
/// Precondition: OPTIONAL MATCH with a single node pattern and RETURN.
/// Expectation: `hir.parts.len() == 1`.
#[test]
fn analyze_optional_match() {
    let hir = analyze("OPTIONAL MATCH (p:Person) RETURN p.name").unwrap();
    check!(hir.parts.len() == 1);
}

#[test]
fn analyze_from_preparsed_query() {
    let query =
        decypher::parse("MATCH (p:Person)-[:KNOWS]->(f) WHERE p.age > 18 RETURN f.name").unwrap();
    let hir = analyze(query).unwrap();
    check!(hir.parts.len() == 1);
}

#[test]
fn analyze_from_preparsed_query_multi_part() {
    let query = decypher::parse(
        "MATCH (p:Person) WITH p, count(*) AS cnt WHERE cnt > 3 RETURN p.name, cnt",
    )
    .unwrap();
    let hir = analyze(query).unwrap();
    check!(hir.parts.len() == 2);
}

#[test]
fn analyze_str_and_query_produce_same_result() {
    let input = "MATCH (p:Person)-[:KNOWS]->(f) RETURN f.name";
    let hir_from_str = analyze(input).unwrap();
    let query = decypher::parse(input).unwrap();
    let hir_from_query = analyze(query).unwrap();
    check!(hir_from_str.parts.len() == hir_from_query.parts.len());
}

#[test]
fn try_from_str_for_query() {
    use std::convert::TryFrom;
    let query = decypher::Query::try_from("MATCH (n) RETURN n").unwrap();
    check!(!query.statements.is_empty());
}

#[test]
fn try_from_str_for_query_invalid() {
    use std::convert::TryFrom;
    let result = decypher::Query::try_from("INVALID !!!");
    check!(result.is_err());
}

#[test]
fn analyze_left_directed_relationship_lowers_to_right_to_left() {
    let hir = analyze("MATCH (a)<-[:T]-(b) RETURN a").unwrap();
    check!(hir.parts.len() == 1);
    let m = find_match_operation(&hir.parts[0].operations);

    check!(m.pattern.relationships.len() == 1);
    check!(m.pattern.relationships[0].direction == RelationshipDirection::RightToLeft);
}

/// Relationships in a chained path must track the correct left (source) node.
///
/// Unit: `lower_pattern_element`
/// Precondition: Two-hop path `(a)-[:E]->(b)-[:F]->(c)`.
/// Expectation: `rel[0].left=0, rel[0].right=1`; `rel[1].left=1, rel[1].right=2`.
#[test]
fn chained_path_relationship_left_indices() {
    let hir = analyze("MATCH (a)-[:E]->(b)-[:F]->(c) RETURN a").unwrap();
    let part = &hir.parts[0];

    let m = find_match_operation(&part.operations);
    let rels = &m.pattern.relationships;
    check!(rels.len() == 2, "expected two relationships");
    check!(rels[0].left == 0, "rel[0].left should be 0");
    check!(rels[0].right == 1, "rel[0].right should be 1");
    check!(rels[1].left == 1, "rel[1].left should be 1, not 0");
    check!(rels[1].right == 2, "rel[1].right should be 2");
}

#[test]
fn analyze_function_call_list_literal_argument() {
    let hir = analyze("RETURN size([1, 2, 3]) AS len").unwrap();
    let project = find_project_operation(&hir.parts[0].operations);

    let call_expr = hir.arenas.expressions.get(project.items[0].expression);
    let list_arg = match &call_expr.kind {
        ExprKind::FunctionCall { args, .. } => {
            check!(args.len() == 1, "expected a single function argument");
            args[0]
        }
        other => panic!("expected function call expression, got {other:?}"),
    };

    let list_expr = hir.arenas.expressions.get(list_arg);
    match &list_expr.kind {
        ExprKind::List(elements) => {
            check!(elements.len() == 3);
            assert_integer_literal(&hir, elements[0], 1);
            assert_integer_literal(&hir, elements[1], 2);
            assert_integer_literal(&hir, elements[2], 3);
        }
        other => panic!("expected list literal argument, got {other:?}"),
    }
}

#[test]
fn analyze_function_call_head_list_literal_argument() {
    let hir = analyze("RETURN head([1, 2, 3]) AS h").unwrap();
    let project = find_project_operation(&hir.parts[0].operations);

    let call_expr = hir.arenas.expressions.get(project.items[0].expression);
    let list_arg = match &call_expr.kind {
        ExprKind::FunctionCall { args, .. } => {
            check!(args.len() == 1, "expected a single function argument");
            args[0]
        }
        other => panic!("expected function call expression, got {other:?}"),
    };

    let list_expr = hir.arenas.expressions.get(list_arg);
    match &list_expr.kind {
        ExprKind::List(elements) => {
            check!(elements.len() == 3);
            assert_integer_literal(&hir, elements[0], 1);
            assert_integer_literal(&hir, elements[1], 2);
            assert_integer_literal(&hir, elements[2], 3);
        }
        other => panic!("expected list literal argument, got {other:?}"),
    }
}

#[test]
fn analyze_function_call_tail_list_literal_argument() {
    let hir = analyze("RETURN tail([1, 2, 3]) AS t").unwrap();
    let project = find_project_operation(&hir.parts[0].operations);

    let call_expr = hir.arenas.expressions.get(project.items[0].expression);
    let list_arg = match &call_expr.kind {
        ExprKind::FunctionCall { args, .. } => {
            check!(args.len() == 1, "expected a single function argument");
            args[0]
        }
        other => panic!("expected function call expression, got {other:?}"),
    };

    let list_expr = hir.arenas.expressions.get(list_arg);
    match &list_expr.kind {
        ExprKind::List(elements) => {
            check!(elements.len() == 3);
            assert_integer_literal(&hir, elements[0], 1);
            assert_integer_literal(&hir, elements[1], 2);
            assert_integer_literal(&hir, elements[2], 3);
        }
        other => panic!("expected list literal argument, got {other:?}"),
    }
}

#[test]
fn analyze_function_call_map_literal_argument() {
    let hir = analyze("RETURN keys({a: 1, b: 2}) AS k").unwrap();
    let project = find_project_operation(&hir.parts[0].operations);

    let call_expr = hir.arenas.expressions.get(project.items[0].expression);
    let map_arg = match &call_expr.kind {
        ExprKind::FunctionCall { args, .. } => {
            check!(args.len() == 1, "expected a single function argument");
            args[0]
        }
        other => panic!("expected function call expression, got {other:?}"),
    };

    let map_expr = hir.arenas.expressions.get(map_arg);
    match &map_expr.kind {
        ExprKind::Map(entries) => {
            check!(entries.len() == 2);
            let key_a = hir.arenas.property_keys.name_of(entries[0].0);
            let key_b = hir.arenas.property_keys.name_of(entries[1].0);
            check!(key_a == Some("a"), "first key should be 'a'");
            check!(key_b == Some("b"), "second key should be 'b'");
            assert_integer_literal(&hir, entries[0].1, 1);
            assert_integer_literal(&hir, entries[1].1, 2);
        }
        other => panic!("expected map literal argument, got {other:?}"),
    }
}

#[test]
fn analyze_standalone_list_literal() {
    let hir = analyze("RETURN [1, 2, 3] AS lst").unwrap();
    let project = find_project_operation(&hir.parts[0].operations);

    let list_expr = hir.arenas.expressions.get(project.items[0].expression);
    match &list_expr.kind {
        ExprKind::List(elements) => {
            check!(elements.len() == 3);
            assert_integer_literal(&hir, elements[0], 1);
            assert_integer_literal(&hir, elements[1], 2);
            assert_integer_literal(&hir, elements[2], 3);
        }
        other => panic!("expected list literal expression, got {other:?}"),
    }
}

/// `RETURN apoc.text.distance(…)` preserves the full qualified name in the interned FunctionId.
///
/// Unit: `analyze()` → `lower_return` → `FunctionId` interning
/// Precondition: Qualified function call with dotted namespace in RETURN.
/// Expectation: `arenas.functions.name_of(id)` returns the full dotted name.
#[test]
fn analyze_return_preserves_qualified_function_name() {
    let hir = analyze("RETURN apoc.text.distance('hello', 'world') AS d").unwrap();
    let ops = &hir.parts[0].operations;

    let function_id = match &ops[0] {
        Operation::Project(op) => match &hir.arenas.expressions.get(op.items[0].expression).kind {
            ExprKind::FunctionCall { function, .. } => *function,
            _ => panic!("expected FunctionCall expression"),
        },
        _ => panic!("expected Project operation"),
    };

    assert_eq!(
        hir.arenas.functions.name_of(function_id),
        Some("apoc.text.distance"),
    );
}

/// `WITH apoc.coll.count(…) AS c` preserves the qualified name through the aggregate interning path.
///
/// Unit: `lower()` with custom registry → `lower_with` aggregate arm → `FunctionId` interning
/// Precondition: Qualified function registered as a custom aggregate via `LowerConfig`.
/// Expectation: `arenas.functions.name_of(aggregate.function)` returns the full dotted name.
#[test]
fn analyze_with_aggregate_preserves_qualified_function_name() {
    let mut config = LowerConfig::default();
    config.aggregates.register("apoc.coll.count");
    let query = decypher::parse("MATCH (n) WITH apoc.coll.count(n.name) AS c RETURN c").unwrap();
    let hir = decypher::hir::lower::lower(&query, &config).unwrap();

    let function_id = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Aggregate(AggregateOp { aggregates, .. }) = op {
                aggregates.first().map(|a| a.function)
            } else {
                None
            }
        })
        .expect("expected an Aggregate operation with at least one aggregate item");

    assert_eq!(
        hir.arenas.functions.name_of(function_id),
        Some("apoc.coll.count"),
    );
}

/// `RETURN apoc.coll.count(…) AS c` preserves the qualified name through the RETURN aggregate path.
///
/// Unit: `lower()` with custom registry → `lower_return` aggregate arm → `FunctionId` interning
/// Precondition: Qualified function registered as a custom aggregate via `LowerConfig`.
/// Expectation: `arenas.functions.name_of(aggregate.function)` returns the full dotted name.
#[test]
fn analyze_return_aggregate_preserves_qualified_function_name() {
    let mut config = LowerConfig::default();
    config.aggregates.register("apoc.coll.count");
    let query = decypher::parse("MATCH (n) RETURN apoc.coll.count(n.name) AS c").unwrap();
    let hir = decypher::hir::lower::lower(&query, &config).unwrap();

    let function_id = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Aggregate(AggregateOp { aggregates, .. }) = op {
                aggregates.first().map(|a| a.function)
            } else {
                None
            }
        })
        .expect("expected an Aggregate operation with at least one aggregate item");

    assert_eq!(
        hir.arenas.functions.name_of(function_id),
        Some("apoc.coll.count"),
    );
}

#[test]
fn function_call_single_binary_argument_lowers_as_single_arg() {
    let hir = analyze("RETURN round(1 + 2) AS r").unwrap();
    let expr_kind = find_first_project_expression(&hir);

    let args = match expr_kind {
        ExprKind::FunctionCall { args, .. } => args,
        other => panic!("expected FunctionCall, got {other:?}"),
    };

    assert_eq!(args.len(), 1, "expected exactly one function argument");
    let arg_expr = &hir.arenas.expressions.get(args[0]).kind;
    match arg_expr {
        ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Add),
        other => panic!("expected binary addition argument, got {other:?}"),
    }
}

#[test]
fn function_call_chained_binary_argument_lowers_as_single_arg() {
    let hir = analyze("RETURN round(3.0 * 4.0 / 5.0) AS r").unwrap();
    let expr_kind = find_first_project_expression(&hir);

    let args = match expr_kind {
        ExprKind::FunctionCall { args, .. } => args,
        other => panic!("expected FunctionCall, got {other:?}"),
    };

    assert_eq!(args.len(), 1, "expected exactly one function argument");
    let arg_expr = &hir.arenas.expressions.get(args[0]).kind;
    match arg_expr {
        ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Divide),
        other => panic!("expected binary division argument, got {other:?}"),
    }
}

#[test]
fn function_call_mixed_precedence_argument_lowers_as_single_arg() {
    // 1 + 2 * 3 must lower as Add(1, Mul(2, 3)), not a flat chain
    let hir = analyze("RETURN round(1 + 2 * 3) AS r").unwrap();
    let expr_kind = find_first_project_expression(&hir);

    let args = match expr_kind {
        ExprKind::FunctionCall { args, .. } => args,
        other => panic!("expected FunctionCall, got {other:?}"),
    };

    assert_eq!(args.len(), 1, "expected exactly one function argument");

    let (add_left, add_right) = match &hir.arenas.expressions.get(args[0]).kind {
        ExprKind::Binary { op, left, right } => {
            assert_eq!(*op, BinaryOp::Add);
            (*left, *right)
        }
        other => panic!("expected top-level Add, got {other:?}"),
    };

    match &hir.arenas.expressions.get(add_left).kind {
        ExprKind::Literal(_) => {}
        other => panic!("expected literal 1 as left operand of Add, got {other:?}"),
    }

    match &hir.arenas.expressions.get(add_right).kind {
        ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Multiply),
        other => panic!("expected Multiply as right operand of Add, got {other:?}"),
    }
}

#[test]
fn function_call_two_binary_arguments_each_lowers_as_one_arg() {
    let hir = analyze("RETURN round(3.0 * 4.0, 1 + 2) AS r").unwrap();
    let expr_kind = find_first_project_expression(&hir);

    let args = match expr_kind {
        ExprKind::FunctionCall { args, .. } => args,
        other => panic!("expected FunctionCall, got {other:?}"),
    };

    assert_eq!(args.len(), 2, "expected exactly two function arguments");

    match &hir.arenas.expressions.get(args[0]).kind {
        ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Multiply),
        other => panic!("expected binary multiply for arg 0, got {other:?}"),
    }
    match &hir.arenas.expressions.get(args[1]).kind {
        ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Add),
        other => panic!("expected binary add for arg 1, got {other:?}"),
    }
}

// ── Query-time variable substitution ─────────────────────────────────────────

/// `CREATE (p:Person {name: $name}) RETURN p` lowers successfully.
///
/// Unit: `analyze()`
/// Precondition: CREATE uses a parameter as a map property value.
/// Expectation: `analyze` returns `Ok` with one query part.
#[test]
fn analyze_create_with_parameter_value() {
    let hir = analyze("CREATE (p:Person {name: $name}) RETURN p").unwrap();
    assert_eq!(hir.parts.len(), 1);
}

/// `MATCH (n) SET n.name = $name RETURN n` lowers successfully.
///
/// Unit: `analyze()`
/// Precondition: SET clause assigns a query parameter to a static property.
/// Expectation: `analyze` returns `Ok` with one query part containing a Set operation.
#[test]
fn analyze_set_property_parameter_value() {
    use decypher::hir::ops::Operation;
    let hir = analyze("MATCH (n) SET n.name = $name RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let has_set = hir.parts[0]
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Set(_)));
    assert!(has_set, "expected a Set operation");
}

/// `MATCH (n) SET n[$key] = $value RETURN n` lowers to a SetDynamicProperty item.
///
/// Unit: `analyze()`
/// Precondition: SET clause uses a parameter as a dynamic property key.
/// Expectation: `analyze` returns `Ok` and the Set operation contains a
///              `SetDynamicProperty` item.
#[test]
fn analyze_set_dynamic_property_key() {
    use decypher::hir::ops::{Operation, SetItem};
    let hir = analyze("MATCH (n) SET n[$key] = $value RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let set_op = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Set(s) = op {
                Some(s)
            } else {
                None
            }
        })
        .expect("expected a Set operation");
    assert_eq!(set_op.items.len(), 1);
    assert!(
        matches!(set_op.items[0], SetItem::SetDynamicProperty { .. }),
        "expected SetDynamicProperty item, got {:?}",
        set_op.items[0]
    );
}

/// `MERGE … ON CREATE SET p.name = $name ON MATCH SET p.lastSeenAt = datetime()` lowers successfully.
///
/// Unit: `analyze()`
/// Precondition: MERGE with ON CREATE and ON MATCH SET actions using parameters.
/// Expectation: `analyze` returns `Ok` with one query part containing a Merge operation.
#[test]
fn analyze_merge_with_parameter_values() {
    use decypher::hir::ops::Operation;
    let hir = analyze(
        "MERGE (p:Person {externalId: $externalId}) \
         ON CREATE SET p.name = $name \
         ON MATCH SET p.lastSeenAt = datetime() \
         RETURN p",
    )
    .unwrap();
    assert_eq!(hir.parts.len(), 1);
    let has_merge = hir.parts[0]
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Merge(_)));
    assert!(has_merge, "expected a Merge operation");
}

/// `MATCH (n) SET n.props[$key] = $value RETURN n` lowers to a SetDynamicProperty item.
///
/// Unit: `analyze()`
/// Precondition: SET clause chains a static property lookup with a dynamic key.
/// Expectation: `analyze` returns `Ok` and the Set operation contains a
///              `SetDynamicProperty` item.
#[test]
fn analyze_set_dynamic_property_key_after_lookup() {
    use decypher::hir::ops::{Operation, SetItem};
    let hir = analyze("MATCH (n) SET n.props[$key] = $value RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let set_op = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Set(s) = op {
                Some(s)
            } else {
                None
            }
        })
        .expect("expected a Set operation");
    assert_eq!(set_op.items.len(), 1);
    assert!(
        matches!(set_op.items[0], SetItem::SetDynamicProperty { .. }),
        "expected SetDynamicProperty item, got {:?}",
        set_op.items[0]
    );
}

/// `MATCH (n) SET n[$k1] = $v1, n[$k2] = $v2 RETURN n` lowers to two SetDynamicProperty items.
///
/// Unit: `analyze()`
/// Precondition: SET clause contains two dynamic-key assignments.
/// Expectation: `analyze` returns `Ok` and the Set operation contains two
///              `SetDynamicProperty` items.
#[test]
fn analyze_set_multiple_dynamic_property_keys() {
    use decypher::hir::ops::{Operation, SetItem};
    let hir = analyze("MATCH (n) SET n[$k1] = $v1, n[$k2] = $v2 RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let set_op = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Set(s) = op {
                Some(s)
            } else {
                None
            }
        })
        .expect("expected a Set operation");
    assert_eq!(set_op.items.len(), 2, "expected two Set items");
    for item in &set_op.items {
        assert!(
            matches!(item, SetItem::SetDynamicProperty { .. }),
            "expected SetDynamicProperty item, got {:?}",
            item
        );
    }
}

/// `MATCH (n) SET n["propName"] = $v RETURN n` lowers to a SetDynamicProperty item.
///
/// Unit: `analyze()`
/// Precondition: SET clause uses a string literal (not a parameter) as the property key.
/// Expectation: `analyze` returns `Ok` and the Set operation contains a
///              `SetDynamicProperty` item.
#[test]
fn analyze_set_dynamic_property_literal_key() {
    use decypher::hir::ops::{Operation, SetItem};
    let hir = analyze("MATCH (n) SET n[\"propName\"] = $v RETURN n").unwrap();
    assert_eq!(hir.parts.len(), 1);
    let set_op = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| {
            if let Operation::Set(s) = op {
                Some(s)
            } else {
                None
            }
        })
        .expect("expected a Set operation");
    assert_eq!(set_op.items.len(), 1);
    assert!(
        matches!(set_op.items[0], SetItem::SetDynamicProperty { .. }),
        "expected SetDynamicProperty item, got {:?}",
        set_op.items[0]
    );
}

/// `reduce` lowers to `ExprKind::Reduce` with the accumulator and element
/// variable bound for the reduction expression (#45).
///
/// Unit: `analyze()`
/// Precondition: `RETURN reduce(acc = 0, x IN [1, 2, 3] | acc + x) AS total`.
/// Expectation: The projection is `ExprKind::Reduce` and lowering succeeds.
#[test]
fn analyze_reduce_binds_accumulator_and_variable() {
    let hir = analyze("RETURN reduce(acc = 0, x IN [1, 2, 3] | acc + x) AS total").unwrap();
    let expr = find_first_project_expression(&hir);
    let ExprKind::Reduce {
        accumulator,
        variable,
        init,
        ..
    } = expr
    else {
        panic!("expected Reduce, got {expr:?}");
    };
    check!(accumulator != variable);
    assert_integer_literal(&hir, *init, 0);
}

/// The `reduce` accumulator and element variable are not visible outside the
/// call (#45).
///
/// Unit: `analyze()`
/// Precondition: A projection references `x` next to a `reduce` binding `x`.
/// Expectation: `analyze` fails with an unresolved variable.
#[test]
fn analyze_reduce_variables_do_not_leak() {
    let err = analyze("RETURN reduce(s = 0, x IN [1] | s + x) AS t, x").unwrap_err();
    check!(err.to_string().contains("unresolved variable `x`"));
}

/// Collections of comprehensions and quantifiers are resolved in the
/// enclosing scope, so `x IN x` refers to the outer `x` (#41).
///
/// Unit: `analyze()`
/// Precondition: Comprehension and `any` whose variable shadows the
///               collection's variable name.
/// Expectation: `analyze` succeeds and the list comprehension's collection
///              is the outer binding, not the element variable.
#[test]
fn analyze_comprehension_collection_uses_enclosing_scope() {
    let hir = analyze("WITH [1, 2] AS x RETURN [x IN x WHERE x > 1] AS r").unwrap();
    let project = find_project_operation(&hir.parts.last().unwrap().operations);
    let expr = &hir.arenas.expressions.get(project.items[0].expression).kind;
    let ExprKind::ListComprehension(lc) = expr else {
        panic!("expected ListComprehension, got {expr:?}");
    };
    let collection = &hir.arenas.expressions.get(lc.collection).kind;
    check!(
        !matches!(collection, ExprKind::Binding(b) if *b == lc.variable),
        "collection must not be the element variable: {collection:?}"
    );

    analyze("WITH [1, 2] AS x RETURN any(x IN x WHERE x > 1) AS r").unwrap();
}

/// A variable bound by `UNWIND` after `WITH` is visible to later clauses
/// (#59).
///
/// Unit: `analyze()`
/// Precondition: `WITH collect(...) AS things UNWIND things AS arr WITH arr[0] …`.
/// Expectation: `analyze` succeeds.
#[test]
fn analyze_unwind_after_with_binds_alias() {
    analyze("UNWIND [[1]] AS l WITH l UNWIND l AS x RETURN x").unwrap();
    analyze("WITH collect([0, 1]) AS things UNWIND things AS arr WITH arr[0] AS e RETURN e")
        .unwrap();
}

/// `YIELD` items of an in-query call are bound for later clauses, and
/// `YIELD … WHERE` becomes a filter (#57).
///
/// Unit: `analyze()`
/// Precondition: In-query calls with plain and aliased yield items.
/// Expectation: Lowering succeeds, the call records its yields, and a
///              `Filter` follows the call when `WHERE` is present.
#[test]
fn analyze_in_query_call_binds_yields() {
    use decypher::hir::ops::{Operation, ProcedureYield};
    let hir = analyze("MATCH (n) CALL foo(n) YIELD x, y AS z WHERE x > 1 RETURN x, z").unwrap();
    let ops = &hir.parts[0].operations;
    let call = ops
        .iter()
        .find_map(|op| match op {
            Operation::CallProcedure(c) => Some(c),
            _ => None,
        })
        .expect("expected a CallProcedure operation");
    check!(call.args.len() == 1);
    let fields: Vec<_> = call
        .yields
        .iter()
        .map(|y: &ProcedureYield| y.field.as_str())
        .collect();
    check!(fields == ["x", "y"]);
    check!(ops.iter().any(|op| matches!(op, Operation::Filter(_))));
}

/// Variables introduced by a pattern comprehension's pattern are visible in
/// its `WHERE` and map expression (#61).
///
/// Unit: `analyze()`
/// Precondition: Pattern comprehensions that use pattern-bound `b` and `r`.
/// Expectation: `analyze` succeeds.
#[test]
fn analyze_pattern_comprehension_binds_pattern_variables() {
    analyze("MATCH (n) RETURN [(n)-[:T]->(b) | b.name] AS list").unwrap();
    analyze("MATCH (n) RETURN [(n)-[r:T]->() WHERE r.x > 1 | r.name] AS list").unwrap();
}

/// Variables introduced by a pattern comprehension do not leak into the
/// enclosing query (#61).
///
/// Unit: `analyze()`
/// Precondition: Projections referencing `b`/`p` next to a pattern
///               comprehension that binds them.
/// Expectation: `analyze` fails with an unresolved variable.
#[test]
fn analyze_pattern_comprehension_variables_do_not_leak() {
    for (query, var) in [
        ("MATCH (n) RETURN [(n)-->(b) | b] AS l, b", "b"),
        ("MATCH (n) WITH [(n)-->(b) | b] AS l RETURN b", "b"),
        ("MATCH (n) RETURN [p = (n)-->() | p] AS l, p", "p"),
    ] {
        let err = analyze(query).unwrap_err();
        check!(
            err.to_string()
                .contains(&format!("unresolved variable `{var}`")),
            "query: {query}, got: {err}"
        );
    }
}

/// Update and projection targets must already be bound (#77).
///
/// Unit: `analyze()`
/// Precondition: `SET`/`REMOVE` label items, `SET x = …` and a map
///               projection on a variable that was never bound.
/// Expectation: `analyze` fails with an unresolved variable.
#[test]
fn analyze_unbound_update_targets_are_reported() {
    for query in [
        "MATCH (n) SET zz:Foo RETURN n",
        "MATCH (n) REMOVE zz:Foo RETURN n",
        "MATCH (n) SET zz = {a: 1} RETURN n",
        "MATCH (n) RETURN zz {.a} AS m",
    ] {
        let err = analyze(query).unwrap_err();
        check!(
            err.to_string().contains("unresolved variable `zz`"),
            "query: {query}, got: {err}"
        );
    }
    analyze("MATCH (n) SET n:Foo, n = {a: 1} RETURN n {.a}").unwrap();
}

/// A pattern variable that is already bound refers to the same entity, so
/// HIR reuses its binding instead of allocating a new one (#79).
///
/// Unit: `analyze()`
/// Precondition: `n` bound by one MATCH and used again in a second MATCH;
///               `a` bound by MATCH and used again in CREATE.
/// Expectation: Exactly one pattern binding exists for each variable.
#[test]
fn analyze_reuses_bindings_for_repeated_pattern_variables() {
    use decypher::hir::binding::BindingKind;
    for (query, var) in [
        ("MATCH (n) MATCH (n)-->(m) RETURN m", "n"),
        ("MATCH (a) CREATE (a)-[:R]->(b) RETURN b", "a"),
    ] {
        let hir = analyze(query).unwrap();
        let pattern_bindings = hir
            .arenas
            .bindings
            .iter()
            .filter(|(_, b)| b.name == var && b.kind == BindingKind::PatternBound)
            .count();
        check!(pattern_bindings == 1, "query: {query}");
    }
}

/// `MERGE p = …` binds the path variable like `MATCH p = …` (#67).
///
/// Unit: `analyze()`
/// Precondition: A path variable introduced by MERGE and returned.
/// Expectation: `analyze` succeeds.
#[test]
fn analyze_merge_binds_path_variable() {
    analyze("MERGE p = (a {num: 1}) RETURN p").unwrap();
    analyze("MERGE (a {num: 1}) MERGE (b {num: 2}) MERGE p = (a)-[:R]->(b) RETURN p").unwrap();
}

/// `CALL { … }` subqueries import outer variables through their leading
/// `WITH`, export their returned columns, and report errors in their body
/// instead of silently lowering to an empty query (#67).
///
/// Unit: `analyze()`
/// Precondition: Correlated and uncorrelated CALL subqueries, a UNION
///               subquery, and subqueries referencing unknown variables.
/// Expectation: Valid queries lower, with the exported columns recorded on
///              the `CallSubquery` operation; invalid ones fail.
#[test]
fn analyze_call_subquery_imports_and_exports() {
    use decypher::hir::ops::Operation;
    let hir = analyze(
        "MATCH (n) CALL { WITH n MATCH (n)-->(m) RETURN count(m) AS c, n AS same } RETURN n, c, same",
    )
    .unwrap();
    let call = hir.parts[0]
        .operations
        .iter()
        .find_map(|op| match op {
            Operation::CallSubquery(c) => Some(c),
            _ => None,
        })
        .expect("expected a CallSubquery operation");
    check!(call.exported_bindings.len() == 2);
    check!(!call.query.parts.is_empty());

    analyze("CALL { MATCH (n) RETURN n } RETURN n").unwrap();
    analyze("CALL { RETURN 1 AS a UNION RETURN 2 AS a } RETURN a").unwrap();

    for (query, var) in [
        ("CALL { RETURN zz AS a } RETURN a", "zz"),
        ("MATCH (n) CALL { WITH n CREATE (m) } RETURN n, m", "m"),
        (
            "MATCH (n) WHERE EXISTS { MATCH (m) WHERE m.v = zz } RETURN n",
            "zz",
        ),
        ("MATCH (n) WHERE EXISTS { (n)-->(m) } RETURN m", "m"),
    ] {
        let err = analyze(query).unwrap_err();
        check!(
            err.to_string()
                .contains(&format!("unresolved variable `{var}`")),
            "query: {query}, got: {err}"
        );
    }
}

/// Aggregates nested in larger expressions are computed by the `Aggregate`
/// operation, and a following `Project` builds the expression from them
/// instead of lowering it to an unknown function (#68).
///
/// Unit: `analyze()`
/// Precondition: Aggregates inside `abs(…)`, `… + 1` and `reduce(… collect(…) …)`,
///               next to a grouping key.
/// Expectation: `Aggregate` holds the inner aggregate calls; a `Project`
///              follows with one item per projected column.
#[test]
fn analyze_nested_aggregates_are_lowered_through_projection() {
    for (query, functions, columns) in [
        ("MATCH (n) RETURN abs(sum(n.x)) AS s", vec!["sum"], 1),
        ("MATCH (n) RETURN count(n) + 1 AS c, n.k", vec!["count"], 2),
        (
            "MATCH (n) RETURN reduce(s = 0, x IN collect(n.v) | s + x) AS t",
            vec!["collect"],
            1,
        ),
    ] {
        let hir = analyze(query).unwrap();
        let ops = &hir.parts[0].operations;
        let aggregate = ops
            .iter()
            .find_map(|op| match op {
                Operation::Aggregate(a) => Some(a),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected an Aggregate operation: {query}"));
        let names: Vec<_> = aggregate
            .aggregates
            .iter()
            .map(|a| hir.arenas.functions.name_of(a.function).unwrap_or_default())
            .collect();
        check!(names == functions, "query: {query}");
        let project = find_project_operation(ops);
        check!(project.items.len() == columns, "query: {query}");
    }
}

/// `DISTINCT` is kept when a projection aggregates.
///
/// Unit: `analyze()`
/// Precondition: `WITH DISTINCT n.k AS k, collect(n) AS ns`.
/// Expectation: The `Project` after the `Aggregate` is distinct.
#[test]
fn analyze_distinct_survives_aggregation() {
    let hir = analyze("MATCH (n) WITH DISTINCT n.k AS k, collect(n) AS ns RETURN k").unwrap();
    let ops = &hir.parts[0].operations;
    check!(ops.iter().any(|op| matches!(op, Operation::Aggregate(_))));
    check!(find_project_operation(ops).distinct);
}

/// A projection that produces the same column name twice is reported (#84).
///
/// Unit: `analyze()`
/// Precondition: Duplicate aliases in RETURN, WITH and a CALL subquery, and
///               two unaliased `count(*)` items.
/// Expectation: `analyze` fails with a redeclared variable; distinct
///              unaliased columns such as `a.name, b.name` are fine.
#[test]
fn analyze_reports_duplicate_projection_columns() {
    for query in [
        "RETURN 1 AS a, 2 AS a",
        "WITH 1 AS a, 2 AS a RETURN a",
        "CALL { RETURN 1 AS a, 2 AS a } RETURN a",
        "MATCH (n) RETURN count(*), count(*)",
    ] {
        let err = analyze(query).unwrap_err();
        check!(
            err.to_string().contains("redeclared"),
            "query: {query}, got: {err}"
        );
    }
    analyze("MATCH (a), (b) RETURN a.name, b.name").unwrap();
    analyze("RETURN 1, 2").unwrap();
}

/// Aliases are not visible to other items of the same projection (#85
/// review): the later `a` is still the matched node, and referencing an
/// alias defined in the same `WITH` fails.
///
/// Unit: `analyze()`
/// Precondition: A RETURN alias shadowing a node variable used again in the
///               same RETURN, and `WITH 1 AS a, a + 1 AS b`.
/// Expectation: The first lowers; the second fails with an unresolved `a`.
#[test]
fn analyze_aliases_are_not_visible_within_their_projection() {
    analyze("MATCH (a) RETURN a.id IS NOT NULL AS a, a IS NOT NULL AS b").unwrap();
    let err = analyze("WITH 1 AS a, a + 1 AS b RETURN b").unwrap_err();
    check!(err.to_string().contains("unresolved variable `a`"));
}

/// Label expressions beyond plain static labels lower to `LabelExpr` instead
/// of an "unsupported" error (#89, #91); static conjunctions and alternations
/// keep the label/type lists.
///
/// Unit: `analyze()`
/// Precondition: Label predicates with `|`, `%` and dynamic labels; node
///               patterns with `:A&B`, `:%`, `:!%`; relationships with
///               `:A|B` and `:!A`.
/// Expectation: `LabelCheck` / `label_expression` / `type_expression` for the
///              rich forms, `NodeLabels` / `labels` / `types` for static ones.
#[test]
fn analyze_label_expressions() {
    use decypher::hir::LabelExpr;
    for query in [
        "MATCH (n) WHERE n:Person|Movie RETURN n",
        "MATCH (n) WHERE n IS % RETURN n",
        "MATCH (n) WHERE n:$('L') RETURN n",
    ] {
        let hir = analyze(query).unwrap();
        check!(
            hir.arenas
                .expressions
                .iter()
                .any(|(_, e)| matches!(e.kind, ExprKind::LabelCheck { .. })),
            "query: {query}"
        );
    }

    let first_match = |query: &str| {
        let hir = analyze(query).unwrap();
        find_match_operation(&hir.parts[0].operations)
            .pattern
            .clone()
    };
    let pattern = first_match("MATCH (n:A&B) RETURN n");
    check!(pattern.nodes[0].labels.len() == 2);
    check!(pattern.nodes[0].label_expression.is_none());

    let pattern = first_match("MATCH (n:!%) RETURN n");
    check!(pattern.nodes[0].labels.is_empty());
    check!(
        pattern.nodes[0].label_expression == Some(LabelExpr::Not(Box::new(LabelExpr::Wildcard)))
    );

    let pattern = first_match("MATCH ()-[r:A|B]->() RETURN r");
    check!(pattern.relationships[0].types.len() == 2);
    check!(pattern.relationships[0].type_expression.is_none());

    let pattern = first_match("MATCH ()-[r:!A]->() RETURN r");
    check!(pattern.relationships[0].types.is_empty());
    check!(matches!(
        pattern.relationships[0].type_expression,
        Some(LabelExpr::Not(_))
    ));
}

/// `WITH` replaces the visible variables by its columns (#96).
///
/// Unit: `analyze()`
/// Precondition: Variables used after a `WITH` that did not project them,
///               including an unaliased `WITH n.name` that does not bind
///               `name`, and `ORDER BY` after `DISTINCT` or aggregation.
/// Expectation: `analyze` fails with an unresolved variable.
#[test]
fn analyze_with_hides_unprojected_variables() {
    for (query, var) in [
        ("MATCH (p)-->(m) WITH p RETURN m", "m"),
        ("MATCH (a), (b) WITH a, b WITH a RETURN b", "b"),
        ("MATCH (n) WITH n.name RETURN name", "name"),
        ("MATCH (n) WITH n MATCH (n)-->(m) WITH n RETURN m", "m"),
        ("MATCH (a), (c) WITH a WITH a ORDER BY c RETURN a", "c"),
        ("MATCH (a) RETURN DISTINCT a.name ORDER BY a.age", "a"),
        (
            "MATCH (me)--(you) WITH count(you.age) AS agg ORDER BY me.age RETURN *",
            "me",
        ),
        ("MATCH (n) WITH count(*) AS c WHERE n.x > 1 RETURN c", "n"),
        (
            "MATCH (n) WITH n.x AS x CALL { RETURN n AS m } RETURN m",
            "n",
        ),
        ("MATCH (n), (m) WITH n RETURN *, m", "m"),
    ] {
        let err = analyze(query).unwrap_err();
        check!(
            err.to_string()
                .contains(&format!("unresolved variable `{var}`")),
            "query: {query}, got: {err}"
        );
    }
}

/// What stays visible across a `WITH` (#96): its columns, everything for
/// `WITH *`, and for its own `WHERE` and `ORDER BY` also the variables it
/// read, as long as aggregation (or, for `ORDER BY`, `DISTINCT`) did not
/// collapse the rows. `ORDER BY` after a narrowing projection may repeat a
/// projected expression.
///
/// Unit: `analyze()`
/// Precondition: Queries the openCypher TCK accepts (`WithWhere7`,
///               `WithOrderBy*`, `ReturnOrderBy*`).
/// Expectation: `analyze` succeeds.
#[test]
fn analyze_with_keeps_projected_and_readable_variables() {
    for query in [
        "MATCH (p)-->(m) WITH p, m.x AS x RETURN p, x",
        "MATCH (p)-->(m) WITH * RETURN p, m",
        "MATCH (p)-->(m) WITH *, m.x AS x RETURN p, m, x",
        "MATCH (a) WITH a.name2 AS name WHERE a.name2 = 'B' RETURN name",
        "MATCH (a) WITH DISTINCT a.name2 AS name WHERE a.name2 = 'B' RETURN name",
        "MATCH (a) WITH a.name AS name ORDER BY a.age RETURN name",
        "MATCH (a) RETURN a.name AS name ORDER BY a.age",
        "MATCH (a) RETURN DISTINCT a.name ORDER BY a.name",
        "MATCH (n) WITH n.k AS k, count(*) AS c WHERE c > 1 RETURN k",
        "MATCH (n) RETURN n.k, count(*) ORDER BY n.k, count(*)",
        "MATCH (n) WITH n CALL { WITH n RETURN n.x AS x } RETURN n, x",
        "MATCH (n) WITH n MATCH (n)-->(m) RETURN n, m",
    ] {
        if let Err(err) = analyze(query) {
            panic!("query: {query}, got: {err}");
        }
    }
}

/// Lowering reports aggregating items that use references which are not
/// grouping keys (#98), deciding what is an aggregate with the registry.
///
/// Unit: `analyze()` / `analyze_with_config()`
/// Precondition: `n.x + count(n)` without and with `n.x` as a grouping key;
///               an ungrouped property next to a registered aggregate.
/// Expectation: `AggregationMix` naming `n.x` where `n.x` is not a grouping
///              key; success otherwise.
#[test]
fn analyze_reports_implicit_grouping_keys() {
    use decypher::ErrorKind;
    for query in [
        "MATCH (n) RETURN n.x + count(n)",
        "MATCH (n) WITH n.x + count(n) AS y RETURN y",
        "CALL { MATCH (n) RETURN n.x + count(n) AS y } RETURN y",
    ] {
        let err = analyze(query).unwrap_err();
        check!(
            err.kind
                == ErrorKind::AggregationMix {
                    non_grouping: vec!["n.x".into()]
                },
            "query: {query}, got: {err}"
        );
    }
    analyze("MATCH (n) RETURN n.x, n.x + count(n) AS y").unwrap();
    analyze("MATCH (n) RETURN n, n.x + count(n) AS y").unwrap();
    analyze("MATCH (n) WITH toUpper(n.name) AS k, count(*) AS c RETURN k, c").unwrap();

    let query = "MATCH (n) RETURN n.x + apoc.agg.percentiles(n.score) AS p";
    analyze(query).unwrap();
    let mut config = LowerConfig::default();
    config.aggregates.register("apoc.agg.percentiles");
    let err = decypher::analyze_with_config(query, &config).unwrap_err();
    check!(matches!(err.kind, ErrorKind::AggregationMix { .. }));
}

/// Variables passed through by `WITH *` / `RETURN *` are grouping keys of an
/// aggregating projection: the `Aggregate` groups by them and a trailing
/// `Project` still outputs them, and expressions over them are not implicit
/// grouping keys (#102 review).
///
/// Unit: `analyze()`
/// Precondition: `*` next to `count(*)`, `count(*) + 1` and `n.x + count(*)`.
/// Expectation: Lowering succeeds; every `Aggregate` groups by `n`.
#[test]
fn analyze_star_variables_are_grouping_keys() {
    for query in [
        "MATCH (n) RETURN DISTINCT *, count(*) AS c",
        "MATCH (n) RETURN *, count(*) + 1 AS c",
        "MATCH (n) RETURN *, n.x + count(*) AS c",
        "MATCH (n) WITH *, n.x + count(*) AS c RETURN n, c",
        "MATCH (n) WITH *, count(*) AS c RETURN n, c",
    ] {
        let hir = analyze(query).unwrap_or_else(|e| panic!("query: {query}, got: {e}"));
        let ops = &hir.parts[0].operations;
        let aggregate = ops
            .iter()
            .find_map(|op| match op {
                Operation::Aggregate(a) => Some(a),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected an Aggregate operation: {query}"));
        let keys: Vec<_> = aggregate
            .grouping_keys
            .iter()
            .map(|key| hir.arenas.bindings.get(key.alias).name.as_str())
            .collect();
        check!(keys == ["n"], "query: {query}");
        if let Some(Operation::Project(project)) = ops
            .iter()
            .rev()
            .find(|op| matches!(op, Operation::Project(_)))
        {
            check!(
                project.items[0].alias == aggregate.grouping_keys[0].alias,
                "query: {query}"
            );
        }
    }
}

