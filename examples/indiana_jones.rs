use decypher::hir::arena::{BindingId, ExprId, HirArenas};
use decypher::hir::expr::{
    BinaryOp, ComparisonOperator as HirComparisonOperator, ExprKind, Literal as HirLiteral,
};
use decypher::hir::ops::MatchOp;
use decypher::hir::pattern::RelationshipDirection;
use decypher::hir::{HirQuery, Operation};
use std::collections::HashMap;

type NodeId = usize;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Null,
    Node(NodeId),
    Edge(NodeId, NodeId, String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, "NULL"),
            Value::Node(id) => write!(f, "Node({})", id),
            Value::Edge(from, to, rel_type) => write!(f, "[:{} {}->{}]", rel_type, from, to),
        }
    }
}

struct Node {
    id: NodeId,
    labels: Vec<String>,
    props: HashMap<String, Value>,
}

struct Edge {
    from: NodeId,
    to: NodeId,
    rel_type: String,
    props: HashMap<String, Value>,
}

struct InMemoryGraph {
    nodes: HashMap<NodeId, Node>,
    edges: Vec<Edge>,
    next_id: NodeId,
}

impl InMemoryGraph {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_id: 0,
        }
    }

    fn add_node(&mut self, labels: &[&str], props: &[(&str, Value)]) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let node = Node {
            id,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            props: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        };
        self.nodes.insert(id, node);
        id
    }

    fn add_edge(&mut self, from: NodeId, to: NodeId, rel_type: &str, props: &[(&str, Value)]) {
        self.edges.push(Edge {
            from,
            to,
            rel_type: rel_type.to_string(),
            props: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        });
    }
}

fn build_graph() -> InMemoryGraph {
    let mut g = InMemoryGraph::new();

    // Actors (Person)
    let indy = g.add_node(
        &["Person"],
        &[("name", Value::String("Harrison Ford".to_string()))],
    );
    let marcus = g.add_node(
        &["Person"],
        &[("name", Value::String("Denholm Elliott".to_string()))],
    );
    let sallah = g.add_node(
        &["Person"],
        &[("name", Value::String("John Rhys-Davies".to_string()))],
    );
    let henry = g.add_node(
        &["Person"],
        &[("name", Value::String("Sean Connery".to_string()))],
    );
    let elsa = g.add_node(
        &["Person"],
        &[("name", Value::String("Alison Doody".to_string()))],
    );
    let donovan = g.add_node(
        &["Person"],
        &[("name", Value::String("Julian Glover".to_string()))],
    );
    let belloq = g.add_node(
        &["Person"],
        &[("name", Value::String("Paul Freeman".to_string()))],
    );
    let mola_ram = g.add_node(
        &["Person"],
        &[("name", Value::String("Amrish Puri".to_string()))],
    );
    let vogel = g.add_node(
        &["Person"],
        &[("name", Value::String("Michael Byrne".to_string()))],
    );
    let spalko = g.add_node(
        &["Person"],
        &[("name", Value::String("Cate Blanchett".to_string()))],
    );

    // Movies
    let raiders = g.add_node(
        &["Movie"],
        &[("name", Value::String("Raiders of the Lost Ark".to_string()))],
    );
    let temple = g.add_node(
        &["Movie"],
        &[("name", Value::String("Temple of Doom".to_string()))],
    );
    let crusade = g.add_node(
        &["Movie"],
        &[("name", Value::String("Last Crusade".to_string()))],
    );
    let crystal = g.add_node(
        &["Movie"],
        &[("name", Value::String("Crystal Skull".to_string()))],
    );

    // Artifacts
    let ark = g.add_node(
        &["Artifact"],
        &[("name", Value::String("Ark of the Covenant".to_string()))],
    );
    let sankara = g.add_node(
        &["Artifact"],
        &[("name", Value::String("Sankara Stones".to_string()))],
    );
    let grail = g.add_node(
        &["Artifact"],
        &[("name", Value::String("Holy Grail".to_string()))],
    );

    // Roles (characters played)
    let role_indy = g.add_node(
        &["Role"],
        &[("name", Value::String("Indiana Jones".to_string()))],
    );
    let role_marcus = g.add_node(
        &["Role"],
        &[("name", Value::String("Marcus Brody".to_string()))],
    );
    let role_sallah = g.add_node(&["Role"], &[("name", Value::String("Sallah".to_string()))]);
    let role_henry = g.add_node(
        &["Role"],
        &[("name", Value::String("Henry Jones Sr.".to_string()))],
    );
    let role_elsa = g.add_node(
        &["Role"],
        &[("name", Value::String("Elsa Schneider".to_string()))],
    );
    let role_donovan = g.add_node(
        &["Role"],
        &[("name", Value::String("Walter Donovan".to_string()))],
    );
    let role_belloq = g.add_node(
        &["Role"],
        &[("name", Value::String("René Belloq".to_string()))],
    );
    let role_mola_ram = g.add_node(
        &["Role"],
        &[("name", Value::String("Mola Ram".to_string()))],
    );
    let role_vogel = g.add_node(
        &["Role"],
        &[("name", Value::String("Colonel Ernst Vogel".to_string()))],
    );
    let role_spalko = g.add_node(
        &["Role"],
        &[("name", Value::String("Irina Spalko".to_string()))],
    );

    // Person -> PLAYS_IN -> Movie
    g.add_edge(indy, raiders, "PLAYS_IN", &[]);
    g.add_edge(indy, temple, "PLAYS_IN", &[]);
    g.add_edge(indy, crusade, "PLAYS_IN", &[]);
    g.add_edge(indy, crystal, "PLAYS_IN", &[]);

    g.add_edge(marcus, raiders, "PLAYS_IN", &[]);
    g.add_edge(marcus, crusade, "PLAYS_IN", &[]);

    g.add_edge(sallah, raiders, "PLAYS_IN", &[]);
    g.add_edge(sallah, crusade, "PLAYS_IN", &[]);

    g.add_edge(henry, crusade, "PLAYS_IN", &[]);
    g.add_edge(elsa, crusade, "PLAYS_IN", &[]);
    g.add_edge(donovan, crusade, "PLAYS_IN", &[]);

    g.add_edge(belloq, raiders, "PLAYS_IN", &[]);
    g.add_edge(mola_ram, temple, "PLAYS_IN", &[]);
    g.add_edge(vogel, crusade, "PLAYS_IN", &[]);
    g.add_edge(spalko, crystal, "PLAYS_IN", &[]);

    // Person -> PLAYS_AS -> Role
    g.add_edge(indy, role_indy, "PLAYS_AS", &[]);
    g.add_edge(marcus, role_marcus, "PLAYS_AS", &[]);
    g.add_edge(sallah, role_sallah, "PLAYS_AS", &[]);
    g.add_edge(henry, role_henry, "PLAYS_AS", &[]);
    g.add_edge(elsa, role_elsa, "PLAYS_AS", &[]);
    g.add_edge(donovan, role_donovan, "PLAYS_AS", &[]);
    g.add_edge(belloq, role_belloq, "PLAYS_AS", &[]);
    g.add_edge(mola_ram, role_mola_ram, "PLAYS_AS", &[]);
    g.add_edge(vogel, role_vogel, "PLAYS_AS", &[]);
    g.add_edge(spalko, role_spalko, "PLAYS_AS", &[]);

    // Role -> SEEKS -> Artifact
    g.add_edge(role_indy, ark, "SEEKS", &[]);
    g.add_edge(role_indy, sankara, "SEEKS", &[]);
    g.add_edge(role_indy, grail, "SEEKS", &[]);

    g.add_edge(role_belloq, ark, "SEEKS", &[]);
    g.add_edge(role_mola_ram, sankara, "SEEKS", &[]);
    g.add_edge(role_vogel, grail, "SEEKS", &[]);

    // Artifact -> IN_MOVIE -> Movie
    g.add_edge(ark, raiders, "IN_MOVIE", &[]);
    g.add_edge(sankara, temple, "IN_MOVIE", &[]);
    g.add_edge(grail, crusade, "IN_MOVIE", &[]);

    g
}

fn interpret(graph: &InMemoryGraph, hir: &HirQuery) -> Vec<Vec<Value>> {
    let mut rows: Vec<HashMap<BindingId, Value>> = Vec::new();

    for part in &hir.parts {
        for op in &part.operations {
            match op {
                Operation::Match(match_op) | Operation::OptionalMatch(match_op) => {
                    let matched = execute_match(graph, match_op, &hir.arenas);
                    rows.extend(matched);
                }
                Operation::Filter(filter_op) => {
                    rows.retain(|ctx| {
                        eval_hir_expr_bool(graph, ctx, filter_op.predicate, &hir.arenas)
                    });
                }
                Operation::Project(project_op) => {
                    let mut new_rows = Vec::new();
                    for ctx in rows {
                        let mut new_ctx = HashMap::new();
                        for item in &project_op.items {
                            let val = eval_hir_expr(graph, &ctx, item.expression, &hir.arenas);
                            new_ctx.insert(item.alias, val);
                        }
                        new_rows.push(new_ctx);
                    }
                    rows = new_rows;
                }
                Operation::Return(_) => {
                    // Terminal — results are already projected
                }
                _ => {}
            }
        }
    }

    // Determine column order from the last Project operation
    let mut col_bindings: Vec<BindingId> = Vec::new();
    for part in &hir.parts {
        for op in &part.operations {
            if let Operation::Project(project_op) = op {
                col_bindings = project_op.items.iter().map(|item| item.alias).collect();
            }
        }
    }

    let mut results = Vec::new();
    for ctx in rows {
        let mut row = Vec::new();
        for binding_id in &col_bindings {
            row.push(ctx.get(binding_id).cloned().unwrap_or(Value::Null));
        }
        results.push(row);
    }

    results
}

fn execute_match(
    graph: &InMemoryGraph,
    match_op: &MatchOp,
    arenas: &HirArenas,
) -> Vec<HashMap<BindingId, Value>> {
    let pattern = &match_op.pattern;
    let mut rows = Vec::new();

    if pattern.relationships.is_empty() && !pattern.nodes.is_empty() {
        // Single node pattern
        let node_pat = &pattern.nodes[0];
        let label_filter = node_pat
            .labels
            .first()
            .and_then(|lid| arenas.labels.name_of(*lid));

        for node in graph.nodes.values() {
            if let Some(lbl) = label_filter
                && !node.labels.iter().any(|l| l == lbl)
            {
                continue;
            }
            let mut ctx = HashMap::new();
            if let Some(binding_id) = node_pat.binding {
                ctx.insert(binding_id, Value::Node(node.id));
            }
            rows.push(ctx);
        }
    } else if pattern.relationships.len() == 1 && pattern.nodes.len() == 2 {
        // Single relationship pattern
        let left_pat = &pattern.nodes[0];
        let right_pat = &pattern.nodes[1];
        let rel_pat = &pattern.relationships[0];

        let left_label = left_pat
            .labels
            .first()
            .and_then(|lid| arenas.labels.name_of(*lid));
        let right_label = right_pat
            .labels
            .first()
            .and_then(|lid| arenas.labels.name_of(*lid));
        let rel_type = rel_pat
            .types
            .first()
            .and_then(|tid| arenas.relationship_types.name_of(*tid));

        for edge in &graph.edges {
            if let Some(rt) = rel_type
                && edge.rel_type != *rt
            {
                continue;
            }

            let candidates = match rel_pat.direction {
                RelationshipDirection::LeftToRight => vec![(edge.from, edge.to)],
                RelationshipDirection::RightToLeft => vec![(edge.to, edge.from)],
                RelationshipDirection::Undirected | RelationshipDirection::Both => {
                    vec![(edge.from, edge.to), (edge.to, edge.from)]
                }
            };

            for (left_id, right_id) in candidates {
                let left_node = graph.nodes.get(&left_id).unwrap();
                let right_node = graph.nodes.get(&right_id).unwrap();

                if let Some(lbl) = left_label
                    && !left_node.labels.iter().any(|l| l == lbl)
                {
                    continue;
                }
                if let Some(lbl) = right_label
                    && !right_node.labels.iter().any(|l| l == lbl)
                {
                    continue;
                }

                let mut ctx = HashMap::new();
                if let Some(bid) = left_pat.binding {
                    ctx.insert(bid, Value::Node(left_id));
                }
                if let Some(bid) = right_pat.binding {
                    ctx.insert(bid, Value::Node(right_id));
                }
                if let Some(bid) = rel_pat.binding {
                    ctx.insert(bid, Value::Edge(edge.from, edge.to, edge.rel_type.clone()));
                }
                rows.push(ctx);
            }
        }
    } else {
        panic!("Only single-node and single-rel patterns are supported");
    }

    // Apply predicates (WHERE expressions folded into MatchOp)
    for pred_id in &match_op.predicates {
        rows.retain(|ctx| eval_hir_expr_bool(graph, ctx, *pred_id, arenas));
    }

    rows
}

fn eval_hir_expr(
    graph: &InMemoryGraph,
    ctx: &HashMap<BindingId, Value>,
    expr_id: ExprId,
    arenas: &HirArenas,
) -> Value {
    let expr = arenas.expressions.get(expr_id);
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            HirLiteral::String(s) => Value::String(s.clone()),
            HirLiteral::Integer(i) => Value::Integer(*i),
            HirLiteral::Float(f) => Value::Float(*f),
            HirLiteral::Boolean(b) => Value::Bool(*b),
            HirLiteral::Null => Value::Null,
        },
        ExprKind::Binding(binding_id) => ctx.get(binding_id).cloned().unwrap_or(Value::Null),
        ExprKind::Property { base, key } => {
            let base_val = eval_hir_expr(graph, ctx, *base, arenas);
            let key_name = arenas.property_keys.name_of(*key).unwrap_or("?");
            match base_val {
                Value::Node(id) => graph
                    .nodes
                    .get(&id)
                    .and_then(|n| n.props.get(key_name))
                    .cloned()
                    .unwrap_or(Value::Null),
                Value::Edge(from, to, rel_type) => graph
                    .edges
                    .iter()
                    .find(|e| e.from == from && e.to == to && e.rel_type == rel_type)
                    .and_then(|e| e.props.get(key_name))
                    .cloned()
                    .unwrap_or(Value::Null),
                _ => Value::Null,
            }
        }
        ExprKind::Comparison { left, operators } => {
            let mut left_val = eval_hir_expr(graph, ctx, *left, arenas);
            for (op, right_id) in operators {
                let right_val = eval_hir_expr(graph, ctx, *right_id, arenas);
                let matched = match op {
                    HirComparisonOperator::Eq => values_equal(&left_val, &right_val),
                    HirComparisonOperator::Ne => !values_equal(&left_val, &right_val),
                    HirComparisonOperator::Lt => value_less_than(&left_val, &right_val),
                    HirComparisonOperator::Gt => value_greater_than(&left_val, &right_val),
                    HirComparisonOperator::Le => {
                        values_equal(&left_val, &right_val)
                            || value_less_than(&left_val, &right_val)
                    }
                    HirComparisonOperator::Ge => {
                        values_equal(&left_val, &right_val)
                            || value_greater_than(&left_val, &right_val)
                    }
                    _ => panic!("Unsupported comparison operator: {:?}", op),
                };
                if !matched {
                    return Value::Bool(false);
                }
                left_val = right_val;
            }
            Value::Bool(true)
        }
        ExprKind::Binary { op, left, right } => {
            let l = eval_hir_expr(graph, ctx, *left, arenas);
            let r = eval_hir_expr(graph, ctx, *right, arenas);
            match op {
                BinaryOp::Add => value_add(&l, &r),
                BinaryOp::Subtract => value_sub(&l, &r),
                BinaryOp::Eq => Value::Bool(values_equal(&l, &r)),
                BinaryOp::Ne => Value::Bool(!values_equal(&l, &r)),
                BinaryOp::Lt => Value::Bool(value_less_than(&l, &r)),
                BinaryOp::Gt => Value::Bool(value_greater_than(&l, &r)),
                BinaryOp::Le => Value::Bool(values_equal(&l, &r) || value_less_than(&l, &r)),
                BinaryOp::Ge => Value::Bool(values_equal(&l, &r) || value_greater_than(&l, &r)),
                _ => panic!("Unsupported binary operator: {:?}", op),
            }
        }
        _ => panic!("Unsupported expression kind: {:?}", expr.kind),
    }
}

fn eval_hir_expr_bool(
    graph: &InMemoryGraph,
    ctx: &HashMap<BindingId, Value>,
    expr_id: ExprId,
    arenas: &HirArenas,
) -> bool {
    matches!(
        eval_hir_expr(graph, ctx, expr_id, arenas),
        Value::Bool(true)
    )
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn value_less_than(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a < b,
        (Value::Float(a), Value::Float(b)) => a < b,
        (Value::String(a), Value::String(b)) => a < b,
        _ => false,
    }
}

fn value_greater_than(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a > b,
        (Value::Float(a), Value::Float(b)) => a > b,
        (Value::String(a), Value::String(b)) => a > b,
        _ => false,
    }
}

fn value_add(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
        (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
        _ => Value::Null,
    }
}

fn value_sub(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a - b),
        (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
        _ => Value::Null,
    }
}

fn print_table(headers: &[String], rows: &[Vec<Value>]) {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let s = cell.to_string();
            if i < widths.len() {
                widths[i] = widths[i].max(s.len());
            }
        }
    }

    let _total_width = widths.iter().sum::<usize>() + widths.len() * 3 + 1;
    let hline = "+".to_string()
        + &widths
            .iter()
            .map(|w| "-".repeat(w + 2))
            .collect::<Vec<_>>()
            .join("+")
        + "+";

    println!("{}", hline);
    print!("|");
    for (i, h) in headers.iter().enumerate() {
        print!(" {:width$} |", h, width = widths[i]);
    }
    println!();
    println!("{}", hline);
    for row in rows {
        print!("|");
        for (i, cell) in row.iter().enumerate() {
            print!(" {:width$} |", cell.to_string(), width = widths[i]);
        }
        println!();
    }
    println!("{}", hline);
}

fn run_query(graph: &InMemoryGraph, query_str: &str) {
    println!("\n--- Query: {} ---", query_str);
    match decypher::analyze(query_str) {
        Ok(hir) => {
            let results = interpret(graph, &hir);
            let headers = extract_headers(&hir);
            print_table(&headers, &results);
        }
        Err(err) => {
            eprintln!("Analyze error: {}", err);
        }
    }
}

fn extract_headers(hir: &HirQuery) -> Vec<String> {
    for part in &hir.parts {
        for op in &part.operations {
            if let Operation::Project(project_op) = op {
                return project_op
                    .items
                    .iter()
                    .map(|item| hir_expr_to_string(item.expression, &hir.arenas))
                    .collect();
            }
        }
    }
    Vec::new()
}

fn hir_expr_to_string(expr_id: ExprId, arenas: &HirArenas) -> String {
    let expr = arenas.expressions.get(expr_id);
    match &expr.kind {
        ExprKind::Binding(binding_id) => {
            let binding = arenas.bindings.get(*binding_id);
            binding.name.clone()
        }
        ExprKind::Property { base, key } => {
            let base_str = hir_expr_to_string(*base, arenas);
            let key_name = arenas.property_keys.name_of(*key).unwrap_or("?");
            format!("{}.{}", base_str, key_name)
        }
        _ => "?".to_string(),
    }
}

fn main() {
    let graph = build_graph();
    println!("=== Indiana Jones Graph ===");
    println!(
        "Loaded {} nodes and {} edges",
        graph.nodes.len(),
        graph.edges.len()
    );

    run_query(&graph, "MATCH (p:Person) RETURN p.name");

    run_query(
        &graph,
        "MATCH (p:Person)-[:PLAYS_IN]->(m:Movie) RETURN p.name, m.name",
    );

    run_query(
        &graph,
        "MATCH (p:Person)-[:PLAYS_IN]->(m:Movie) WHERE m.name = \"Last Crusade\" RETURN p.name",
    );

    run_query(
        &graph,
        "MATCH (r:Role)-[:SEEKS]->(a:Artifact) RETURN r.name, a.name",
    );
}
