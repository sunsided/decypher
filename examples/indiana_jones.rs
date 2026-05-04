use cypher::ast::expr::{ComparisonOperator, Expression, Literal};
use cypher::ast::pattern::{
    LabelExpression, NodePattern, PatternElement, RelationshipDirection, RelationshipPattern,
};
use cypher::ast::query::{Query, QueryBody, ReadingClause, SinglePartBody, SingleQueryKind};
use std::collections::HashMap;

type NodeId = usize;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, "NULL"),
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

    // ── Actors (Person) ────────────────────────────────────────────────
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

    // ── Movies ─────────────────────────────────────────────────────────
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

    // ── Artifacts ──────────────────────────────────────────────────────
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

    // ── Roles (characters played) ──────────────────────────────────────
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

    // ── Person -> PLAYS_IN -> Movie ────────────────────────────────────
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

    // ── Person -> PLAYS_AS -> Role ─────────────────────────────────────
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

    // ── Role -> SEEKS -> Artifact ──────────────────────────────────────
    g.add_edge(role_indy, ark, "SEEKS", &[]);
    g.add_edge(role_indy, sankara, "SEEKS", &[]);
    g.add_edge(role_indy, grail, "SEEKS", &[]);

    g.add_edge(role_belloq, ark, "SEEKS", &[]);
    g.add_edge(role_mola_ram, sankara, "SEEKS", &[]);
    g.add_edge(role_vogel, grail, "SEEKS", &[]);

    // ── Artifact -> IN_MOVIE -> Movie ──────────────────────────────────
    g.add_edge(ark, raiders, "IN_MOVIE", &[]);
    g.add_edge(sankara, temple, "IN_MOVIE", &[]);
    g.add_edge(grail, crusade, "IN_MOVIE", &[]);

    g
}

fn interpret(graph: &InMemoryGraph, query: &Query) -> Vec<Vec<Value>> {
    let statement = &query.statements[0];
    let QueryBody::SingleQuery(sq) = statement else {
        panic!("Only single queries are supported");
    };
    let SingleQueryKind::SinglePart(sp) = &sq.kind else {
        panic!("Only single-part queries are supported");
    };

    let match_clause = sp.reading_clauses.iter().find_map(|rc| match rc {
        ReadingClause::Match(m) => Some(m),
        _ => None,
    });

    let QueryBody::SingleQuery(ret_sq) = statement else {
        unreachable!()
    };
    let SingleQueryKind::SinglePart(ret_sp) = &ret_sq.kind else {
        unreachable!()
    };
    let SinglePartBody::Return(ret) = &ret_sp.body else {
        panic!("Only RETURN body is supported");
    };

    let mut rows: Vec<HashMap<String, Binding>> = Vec::new();

    if let Some(m) = match_clause {
        let part = m.pattern.parts.first().expect("MATCH must have a pattern");
        let anonymous = &part.anonymous;
        let PatternElement::Path { start, chains } = &anonymous.element else {
            panic!("Only path patterns are supported");
        };

        let start_label = get_node_label(start);
        let start_var = get_node_variable(start);

        if chains.is_empty() {
            for node in graph.nodes.values() {
                if let Some(ref lbl) = start_label {
                    if !node.labels.contains(lbl) {
                        continue;
                    }
                }
                let mut ctx = HashMap::new();
                if let Some(ref v) = start_var {
                    ctx.insert(v.clone(), Binding::Node(node.id));
                }
                if evaluate_where(graph, &ctx, m.where_clause.as_ref()) {
                    rows.push(ctx);
                }
            }
        } else if chains.len() == 1 {
            let chain = &chains[0];
            let end = &chain.node;
            let end_label = get_node_label(end);
            let end_var = get_node_variable(end);
            let rel_var = get_rel_variable(&chain.relationship);
            let rel_type = get_rel_type(&chain.relationship);
            let dir = &chain.relationship.direction;

            for edge in &graph.edges {
                if let Some(ref rt) = rel_type {
                    if edge.rel_type != *rt {
                        continue;
                    }
                }

                let (from_node_id, to_node_id) = match dir {
                    RelationshipDirection::Left => (edge.to, edge.from),
                    RelationshipDirection::Right => (edge.from, edge.to),
                    RelationshipDirection::Both | RelationshipDirection::Undirected => {
                        let from_node = graph.nodes.get(&edge.from).unwrap();
                        let to_node = graph.nodes.get(&edge.to).unwrap();

                        let start_ok = start_label
                            .as_ref()
                            .map_or(true, |l| from_node.labels.contains(l));
                        let end_ok = end_label
                            .as_ref()
                            .map_or(true, |l| to_node.labels.contains(l));

                        if start_ok && end_ok {
                            let mut ctx1 = HashMap::new();
                            if let Some(ref v) = start_var {
                                ctx1.insert(v.clone(), Binding::Node(edge.from));
                            }
                            if let Some(ref v) = end_var {
                                ctx1.insert(v.clone(), Binding::Node(edge.to));
                            }
                            if let Some(ref v) = rel_var {
                                ctx1.insert(
                                    v.clone(),
                                    Binding::Edge(edge.from, edge.to, edge.rel_type.clone()),
                                );
                            }
                            if evaluate_where(graph, &ctx1, m.where_clause.as_ref()) {
                                rows.push(ctx1);
                            }
                        }

                        let start_ok2 = start_label
                            .as_ref()
                            .map_or(true, |l| to_node.labels.contains(l));
                        let end_ok2 = end_label
                            .as_ref()
                            .map_or(true, |l| from_node.labels.contains(l));

                        if start_ok2 && end_ok2 {
                            let mut ctx2 = HashMap::new();
                            if let Some(ref v) = start_var {
                                ctx2.insert(v.clone(), Binding::Node(edge.to));
                            }
                            if let Some(ref v) = end_var {
                                ctx2.insert(v.clone(), Binding::Node(edge.from));
                            }
                            if let Some(ref v) = rel_var {
                                ctx2.insert(
                                    v.clone(),
                                    Binding::Edge(edge.to, edge.from, edge.rel_type.clone()),
                                );
                            }
                            if evaluate_where(graph, &ctx2, m.where_clause.as_ref()) {
                                rows.push(ctx2);
                            }
                        }
                        continue;
                    }
                };

                let from_node = graph.nodes.get(&from_node_id).unwrap();
                let to_node = graph.nodes.get(&to_node_id).unwrap();
                let start_ok = start_label
                    .as_ref()
                    .map_or(true, |l| from_node.labels.contains(l));
                let end_ok = end_label
                    .as_ref()
                    .map_or(true, |l| to_node.labels.contains(l));

                if start_ok && end_ok {
                    let mut ctx = HashMap::new();
                    if let Some(ref v) = start_var {
                        ctx.insert(v.clone(), Binding::Node(from_node_id));
                    }
                    if let Some(ref v) = end_var {
                        ctx.insert(v.clone(), Binding::Node(to_node_id));
                    }
                    if let Some(ref v) = rel_var {
                        ctx.insert(
                            v.clone(),
                            Binding::Edge(edge.from, edge.to, edge.rel_type.clone()),
                        );
                    }
                    if evaluate_where(graph, &ctx, m.where_clause.as_ref()) {
                        rows.push(ctx);
                    }
                }
            }
        } else {
            panic!("Only single-relationship patterns are supported");
        }
    } else {
        panic!("No MATCH clause found");
    }

    let mut results = Vec::new();
    for ctx in rows {
        let mut row = Vec::new();
        for item in &ret.items {
            row.push(eval_expr(graph, &ctx, &item.expression));
        }
        results.push(row);
    }

    results
}

#[derive(Debug, Clone)]
enum Binding {
    Node(NodeId),
    Edge(NodeId, NodeId, String),
}

fn get_node_label(node: &NodePattern) -> Option<String> {
    node.labels.first().and_then(|l| match l {
        LabelExpression::Static(s) => Some(s.name.clone()),
        _ => None,
    })
}

fn get_node_variable(node: &NodePattern) -> Option<String> {
    node.variable.as_ref().map(|v| v.name.name.clone())
}

fn get_rel_variable(rel: &RelationshipPattern) -> Option<String> {
    rel.detail
        .as_ref()
        .and_then(|d| d.variable.as_ref().map(|v| v.name.name.clone()))
}

fn get_rel_type(rel: &RelationshipPattern) -> Option<String> {
    rel.detail.as_ref().and_then(|d| {
        d.types.as_ref().and_then(|t| match t {
            LabelExpression::Static(s) => Some(s.name.clone()),
            _ => None,
        })
    })
}

fn evaluate_where(
    graph: &InMemoryGraph,
    ctx: &HashMap<String, Binding>,
    where_clause: Option<&Expression>,
) -> bool {
    let Some(expr) = where_clause else {
        return true;
    };
    match expr {
        Expression::Comparison { lhs, operators, .. } => {
            let left_val = eval_expr(graph, ctx, lhs);
            for (op, rhs) in operators {
                let right_val = eval_expr(graph, ctx, rhs);
                match op {
                    ComparisonOperator::Eq => {
                        if !values_equal(&left_val, &right_val) {
                            return false;
                        }
                    }
                    _ => panic!("Only = operator is supported in WHERE"),
                }
            }
            true
        }
        _ => panic!("Unsupported WHERE expression"),
    }
}

fn eval_expr(graph: &InMemoryGraph, ctx: &HashMap<String, Binding>, expr: &Expression) -> Value {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::String(s) => Value::String(s.value.clone()),
            Literal::Number(n) => match n {
                cypher::ast::expr::NumberLiteral::Integer(i) => Value::Integer(*i),
                cypher::ast::expr::NumberLiteral::Float(f) => Value::Float(*f),
            },
            Literal::Boolean(b) => Value::Bool(*b),
            Literal::Null => Value::Null,
            _ => panic!("Unsupported literal"),
        },
        Expression::Variable(v) => {
            let name = &v.name.name;
            if let Some(binding) = ctx.get(name) {
                match binding {
                    Binding::Node(id) => {
                        let node = graph.nodes.get(id).unwrap();
                        if let Some(v) = node.props.get("name") {
                            v.clone()
                        } else {
                            Value::Null
                        }
                    }
                    Binding::Edge(from, to, rel_type) => {
                        Value::String(format!("[:{} {}->{}]", rel_type, from, to))
                    }
                }
            } else {
                Value::Null
            }
        }
        Expression::PropertyLookup { base, property, .. } => {
            let prop_name = &property.name.name;
            match base.as_ref() {
                Expression::Variable(v) => {
                    let var_name = &v.name.name;
                    if let Some(binding) = ctx.get(var_name) {
                        match binding {
                            Binding::Node(id) => {
                                let node = graph.nodes.get(id).unwrap();
                                node.props.get(prop_name).cloned().unwrap_or(Value::Null)
                            }
                            Binding::Edge(from, to, rel_type) => {
                                let edge = graph
                                    .edges
                                    .iter()
                                    .find(|e| {
                                        e.from == *from && e.to == *to && e.rel_type == *rel_type
                                    })
                                    .expect("Edge must exist");
                                edge.props.get(prop_name).cloned().unwrap_or(Value::Null)
                            }
                        }
                    } else {
                        Value::Null
                    }
                }
                _ => panic!("Unsupported base expression in property lookup"),
            }
        }
        _ => panic!("Unsupported expression in RETURN"),
    }
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
    match cypher::parse(query_str) {
        Ok(query) => {
            let results = interpret(graph, &query);

            let query_clone = match cypher::parse(query_str) {
                Ok(q) => q,
                Err(_) => {
                    println!("Parse error");
                    return;
                }
            };
            let statement = &query_clone.statements[0];
            let QueryBody::SingleQuery(sq) = statement else {
                println!("Not a single query");
                return;
            };
            let SingleQueryKind::SinglePart(sp) = &sq.kind else {
                println!("Not a single-part query");
                return;
            };
            let SinglePartBody::Return(ret) = &sp.body else {
                println!("No RETURN clause");
                return;
            };

            let headers: Vec<String> = ret
                .items
                .iter()
                .map(|item| expr_to_string(&item.expression))
                .collect();

            print_table(&headers, &results);
        }
        Err(err) => {
            eprintln!("Parse error: {}", err);
        }
    }
}

fn expr_to_string(expr: &Expression) -> String {
    match expr {
        Expression::Variable(v) => v.name.name.clone(),
        Expression::PropertyLookup { base, property, .. } => {
            format!("{}.{} {}", expr_to_string(base), property.name.name, "")
        }
        _ => "?".to_string(),
    }
    .trim()
    .to_string()
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
