//! CST → typed AST lowering.
//!
//! Walks the rowan-based CST wrappers produced by Phase B and builds the
//! existing `ast::query::Query` tree. No public API change — `parse()`
//! stays on pest until Phase D.

use crate::ast::build_shared::{decode_string_content, parse_double, parse_integer};
use crate::error::{CypherError, ErrorKind, Result, Span};
use crate::syntax::ast::AstNode;
use crate::syntax::SyntaxKind;

// ── CST module aliases (source) ─────────────────────────────────────
use cst_c::*;

mod cst_c {
    pub use crate::syntax::ast::clauses::*;
    pub use crate::syntax::ast::expressions;
    pub use crate::syntax::ast::expressions::*;
    pub use crate::syntax::ast::patterns;
    pub use crate::syntax::ast::patterns::*;
    pub use crate::syntax::ast::projection::*;
    pub use crate::syntax::ast::schema as schema_cst;
    pub use crate::syntax::ast::top_level;
    pub use crate::syntax::ast::top_level::*;
    pub use crate::syntax::ast::AstNode;
}

// ── AST module aliases (target) ─────────────────────────────────────
mod ast_c {
    pub use crate::ast::clause::*;
    pub use crate::ast::expr::*;
    pub use crate::ast::names::*;
    pub use crate::ast::pattern::*;
    pub use crate::ast::procedure::*;
    pub use crate::ast::query::*;
    pub use crate::ast::schema::*;
}

// ── Shared helpers ───────────────────────────────────────────────────

fn span_of(node: &rowan::SyntaxNode<crate::syntax::CypherLang>) -> Span {
    let r = node.text_range();
    Span::new(r.start().into(), r.end().into())
}

fn symbolic_name_text(sym: &SymbolicName) -> String {
    if let Some(ident) = sym.ident_token() {
        ident.unescape()
    } else {
        String::new()
    }
}

fn internal(msg: &str, sp: Span) -> CypherError {
    CypherError {
        kind: ErrorKind::Internal {
            message: msg.into(),
        },
        span: sp,
        source_label: None,
        notes: Vec::new(),
        source: None,
    }
}

// ── Entry point ──────────────────────────────────────────────────────

pub fn build_source_file(src: SourceFile) -> Result<ast_c::Query> {
    let sp = span_of(src.syntax());
    let mut statements = Vec::new();

    for cmd in src.schema_commands() {
        statements.push(ast_c::QueryBody::SchemaCommand(build_schema_command(cmd)?));
    }

    for stmt in src.statements() {
        statements.extend(build_statement(stmt)?);
    }

    if statements.is_empty() {
        return Err(internal("empty source file", sp));
    }

    Ok(ast_c::Query {
        statements,
        span: sp,
    })
}

fn build_statement(stmt: Statement) -> Result<Vec<ast_c::QueryBody>> {
    let clauses: Vec<_> = stmt.clauses().collect();
    if clauses.is_empty() {
        return Err(internal("empty statement", span_of(stmt.syntax())));
    }

    if clauses.len() == 1 {
        match &clauses[0] {
            Clause::Show(c) => return Ok(vec![ast_c::QueryBody::Show(build_show(c.clone())?)]),
            Clause::Use(c) => return Ok(vec![ast_c::QueryBody::Use(build_use(c.clone())?)]),
            Clause::StandaloneCall(c) => {
                return Ok(vec![ast_c::QueryBody::Standalone(build_standalone_call(
                    c.clone(),
                )?)])
            }
            _ => {}
        }
    }

    let unions: Vec<_> = stmt
        .syntax()
        .children()
        .filter_map(|n| Union::cast(n))
        .collect();

    if !unions.is_empty() {
        let regular = build_regular_query(&clauses, &unions)?;
        if regular.unions.is_empty() {
            Ok(vec![ast_c::QueryBody::SingleQuery(regular.single_query)])
        } else {
            Ok(vec![ast_c::QueryBody::Regular(regular)])
        }
    } else {
        let single = build_single_query_from_clauses(clauses)?;
        Ok(vec![ast_c::QueryBody::SingleQuery(single)])
    }
}

fn build_regular_query(clauses: &[Clause], unions: &[Union]) -> Result<ast_c::RegularQuery> {
    let single_query = build_single_query_from_clauses(clauses.to_vec())?;
    let mut result_unions = Vec::new();

    for union_node in unions {
        let all = union_node.all_token().is_some();
        let union_clauses: Vec<_> = union_node.clauses().collect();
        let inner_unions: Vec<_> = union_node.inner_unions().collect();
        let mut sq = build_single_query_from_clauses(union_clauses)?;

        // Handle nested unions
        for nested in inner_unions {
            let n_all = nested.all_token().is_some();
            let n_clauses: Vec<_> = nested.clauses().collect();
            let n_sq = build_single_query_from_clauses(n_clauses)?;
            let _ = n_all;
            sq = n_sq;
        }

        result_unions.push(ast_c::Union {
            all,
            single_query: sq,
            span: span_of(union_node.syntax()),
        });
    }

    Ok(ast_c::RegularQuery {
        single_query,
        unions: result_unions,
    })
}

fn build_single_query_from_clauses(clauses: Vec<Clause>) -> Result<ast_c::SingleQuery> {
    let with_indices: Vec<_> = clauses
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, Clause::With(_)))
        .map(|(i, _)| i)
        .collect();

    let total = clauses.len();
    let is_multipart_with = |idx: usize| -> bool { idx < total - 1 };

    if with_indices.is_empty() {
        let mut reading = Vec::new();
        let mut updating = Vec::new();
        let mut ret = None;

        for c in clauses {
            match c {
                Clause::Match(m) => reading.push(ast_c::ReadingClause::Match(build_match(m)?)),
                Clause::Unwind(u) => reading.push(ast_c::ReadingClause::Unwind(build_unwind(u)?)),
                Clause::InQueryCall(ic) => {
                    reading.push(ast_c::ReadingClause::InQueryCall(build_in_query_call(ic)?))
                }
                Clause::CallSubquery(cs) => {
                    reading.push(ast_c::ReadingClause::CallSubquery(build_call_subquery(cs)?))
                }
                Clause::Create(c) => updating.push(ast_c::UpdatingClause::Create(build_create(c)?)),
                Clause::Merge(m) => updating.push(ast_c::UpdatingClause::Merge(build_merge(m)?)),
                Clause::Set(s) => updating.push(ast_c::UpdatingClause::Set(build_set(s)?)),
                Clause::Remove(r) => updating.push(ast_c::UpdatingClause::Remove(build_remove(r)?)),
                Clause::Delete(d) => updating.push(ast_c::UpdatingClause::Delete(build_delete(d)?)),
                Clause::Foreach(f) => {
                    updating.push(ast_c::UpdatingClause::Foreach(build_foreach(f)?))
                }
                Clause::Return(r) => ret = Some(build_return(r)?),
                Clause::With(_) => {}
                Clause::Where(_) => {}
                Clause::Show(_) | Clause::Use(_) | Clause::StandaloneCall(_) => {}
            }
        }

        let body = if !updating.is_empty() {
            ast_c::SinglePartBody::Updating {
                updating,
                return_clause: ret,
            }
        } else {
            ast_c::SinglePartBody::Return(ret.ok_or_else(|| {
                internal("single-part query must end with RETURN", Span::new(0, 0))
            })?)
        };

        Ok(ast_c::SingleQuery {
            kind: ast_c::SingleQueryKind::SinglePart(ast_c::SinglePartQuery {
                reading_clauses: reading,
                body,
            }),
        })
    } else {
        let mut parts = Vec::new();
        let mut final_part = None;
        let mut reading = Vec::new();
        let mut updating = Vec::new();

        for (i, c) in clauses.into_iter().enumerate() {
            match c {
                Clause::With(w) if is_multipart_with(i) => {
                    let wc = build_with(w)?;
                    parts.push(ast_c::MultiPartQueryPart {
                        reading_clauses: std::mem::take(&mut reading),
                        updating_clauses: std::mem::take(&mut updating),
                        with: wc,
                    });
                }
                Clause::Match(m) => reading.push(ast_c::ReadingClause::Match(build_match(m)?)),
                Clause::Unwind(u) => reading.push(ast_c::ReadingClause::Unwind(build_unwind(u)?)),
                Clause::InQueryCall(ic) => {
                    reading.push(ast_c::ReadingClause::InQueryCall(build_in_query_call(ic)?))
                }
                Clause::CallSubquery(cs) => {
                    reading.push(ast_c::ReadingClause::CallSubquery(build_call_subquery(cs)?))
                }
                Clause::Create(c) => updating.push(ast_c::UpdatingClause::Create(build_create(c)?)),
                Clause::Merge(m) => updating.push(ast_c::UpdatingClause::Merge(build_merge(m)?)),
                Clause::Set(s) => updating.push(ast_c::UpdatingClause::Set(build_set(s)?)),
                Clause::Remove(r) => updating.push(ast_c::UpdatingClause::Remove(build_remove(r)?)),
                Clause::Delete(d) => updating.push(ast_c::UpdatingClause::Delete(build_delete(d)?)),
                Clause::Foreach(f) => {
                    updating.push(ast_c::UpdatingClause::Foreach(build_foreach(f)?))
                }
                Clause::Return(r) => {
                    let ret = build_return(r)?;
                    final_part = Some(ast_c::SinglePartQuery {
                        reading_clauses: std::mem::take(&mut reading),
                        body: ast_c::SinglePartBody::Return(ret),
                    });
                }
                Clause::With(w) => {
                    let wc = build_with(w)?;
                    final_part = Some(ast_c::SinglePartQuery {
                        reading_clauses: std::mem::take(&mut reading),
                        body: ast_c::SinglePartBody::Return(ast_c::Return {
                            distinct: wc.distinct,
                            items: wc.items,
                            order: wc.order,
                            skip: wc.skip,
                            limit: wc.limit,
                            span: wc.span,
                        }),
                    });
                }
                Clause::Where(_) => {}
                Clause::Show(_) | Clause::Use(_) | Clause::StandaloneCall(_) => {}
            }
        }

        let final_part = final_part
            .ok_or_else(|| internal("multi-part query missing final part", Span::new(0, 0)))?;

        Ok(ast_c::SingleQuery {
            kind: ast_c::SingleQueryKind::MultiPart(ast_c::MultiPartQuery { parts, final_part }),
        })
    }
}

// ── Clauses ──────────────────────────────────────────────────────────

fn build_match(c: MatchClause) -> Result<ast_c::Match> {
    let sp = span_of(c.syntax());
    let optional = c.optional_token().is_some();
    let pattern = c
        .pattern()
        .map(build_pattern)
        .transpose()?
        .ok_or_else(|| internal("missing pattern in MATCH", sp))?;
    let where_clause = c
        .where_clause()
        .and_then(|w| w.expr())
        .map(|e| build_expression(e))
        .transpose()?;
    Ok(ast_c::Match {
        optional,
        pattern,
        where_clause,
        span: sp,
    })
}

fn build_unwind(c: UnwindClause) -> Result<ast_c::Unwind> {
    let sp = span_of(c.syntax());
    let expr = c
        .expr()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing expression in UNWIND", sp))?;
    let variable = c
        .as_name()
        .map(|v| build_top_variable(v))
        .ok_or_else(|| internal("missing variable in UNWIND", sp))?;
    Ok(ast_c::Unwind {
        expression: expr,
        variable,
        span: sp,
    })
}

fn build_create(c: CreateClause) -> Result<ast_c::Create> {
    let sp = span_of(c.syntax());
    let pattern = c
        .pattern()
        .map(|p| build_pattern(p))
        .transpose()?
        .ok_or_else(|| internal("missing pattern in CREATE", sp))?;
    Ok(ast_c::Create { pattern, span: sp })
}

fn build_merge(c: MergeClause) -> Result<ast_c::Merge> {
    let sp = span_of(c.syntax());
    let pattern = c
        .pattern()
        .map(|p| build_pattern_part(p))
        .transpose()?
        .ok_or_else(|| internal("missing pattern in MERGE", sp))?;
    let actions: Result<Vec<_>> = c.actions().map(|a| build_merge_action(a)).collect();
    Ok(ast_c::Merge {
        pattern,
        actions: actions?,
        span: sp,
    })
}

fn build_merge_action(a: MergeAction) -> Result<ast_c::MergeAction> {
    let sp = span_of(a.syntax());
    let on_match = a
        .match_or_create_token()
        .map(|t| t.kind() == SyntaxKind::KW_MATCH)
        .unwrap_or(false);
    let items: Result<Vec<_>> = a.set_items().map(|s| build_set_item(s)).collect();
    Ok(ast_c::MergeAction {
        on_match,
        set_items: items?,
        span: sp,
    })
}

fn build_set(c: SetClause) -> Result<ast_c::Set> {
    let sp = span_of(c.syntax());
    let items: Result<Vec<_>> = c.items().map(|i| build_set_item(i)).collect();
    Ok(ast_c::Set {
        items: items?,
        span: sp,
    })
}

fn build_set_item(item: SetItem) -> Result<ast_c::SetItem> {
    let sp = span_of(item.syntax());

    if let Some(labels) = item.node_labels() {
        if let Some(_prop_expr) = item.property_expr() {
            let ast_labels: Result<Vec<_>> = labels
                .labels()
                .map(|l| {
                    l.name()
                        .and_then(|ln| ln.symbolic_name())
                        .map(|s| ast_c::SymbolicName {
                            name: symbolic_name_text(&s),
                            span: span_of(s.syntax()),
                        })
                        .ok_or_else(|| internal("missing label name", sp))
                })
                .collect();
            let v = ast_c::Variable {
                name: ast_c::SymbolicName {
                    name: String::new(),
                    span: sp,
                },
            };
            return Ok(ast_c::SetItem::Labels {
                variable: v,
                labels: ast_labels?,
            });
        }
    }

    let property = item
        .property_expr()
        .ok_or_else(|| internal("missing property expression in SET item", sp))?;
    let value = item
        .value_expr()
        .ok_or_else(|| internal("missing value expression in SET item", sp))?;
    let value_ast = build_expression(value)?;
    let operator = if item.plus_eq_token().is_some() {
        ast_c::SetOperator::Add
    } else {
        ast_c::SetOperator::Assign
    };

    let prop_ast = build_expression(property)?;
    match &prop_ast {
        ast_c::Expression::Variable(v) => Ok(ast_c::SetItem::Variable {
            variable: v.clone(),
            value: value_ast,
            operator,
        }),
        _ => Ok(ast_c::SetItem::Property {
            property: prop_ast,
            value: value_ast,
            operator,
        }),
    }
}

fn build_delete(c: DeleteClause) -> Result<ast_c::Delete> {
    let sp = span_of(c.syntax());
    let detach = c.detach_token().is_some();
    let targets: Result<Vec<_>> = c.exprs().map(|e| build_expression(e)).collect();
    Ok(ast_c::Delete {
        detach,
        targets: targets?,
        span: sp,
    })
}

fn build_remove(c: RemoveClause) -> Result<ast_c::Remove> {
    let sp = span_of(c.syntax());
    let items: Result<Vec<_>> = c.items().map(|i| build_remove_item(i)).collect();
    Ok(ast_c::Remove {
        items: items?,
        span: sp,
    })
}

fn build_remove_item(item: RemoveItem) -> Result<ast_c::RemoveItem> {
    let sp = span_of(item.syntax());
    if let Some(labels) = item.node_labels() {
        if let Some(_prop_expr) = item.property_expr() {
            let ast_labels: Result<Vec<_>> = labels
                .labels()
                .map(|l| {
                    l.name()
                        .and_then(|ln| ln.symbolic_name())
                        .map(|s| ast_c::SymbolicName {
                            name: symbolic_name_text(&s),
                            span: span_of(s.syntax()),
                        })
                        .ok_or_else(|| internal("missing label name", sp))
                })
                .collect();
            let v = ast_c::Variable {
                name: ast_c::SymbolicName {
                    name: String::new(),
                    span: sp,
                },
            };
            return Ok(ast_c::RemoveItem::Labels {
                variable: v,
                labels: ast_labels?,
            });
        }
    }
    let prop = item
        .property_expr()
        .ok_or_else(|| internal("missing property in REMOVE item", sp))?;
    let prop_ast = build_expression(prop)?;
    Ok(ast_c::RemoveItem::Property(prop_ast))
}

fn build_with(c: WithClause) -> Result<ast_c::With> {
    let sp = span_of(c.syntax());
    let proj = c
        .projection_body()
        .map(|b| build_projection_body(b))
        .transpose()?
        .ok_or_else(|| internal("missing projection in WITH", sp))?;
    let where_clause = c
        .where_clause()
        .and_then(|w| w.expr())
        .map(|e| build_expression(e))
        .transpose()?;
    Ok(ast_c::With {
        distinct: proj.distinct,
        items: proj.items,
        order: proj.order,
        skip: proj.skip,
        limit: proj.limit,
        where_clause,
        span: sp,
    })
}

fn build_return(c: ReturnClause) -> Result<ast_c::Return> {
    let sp = span_of(c.syntax());
    let proj = c
        .projection_body()
        .map(|b| build_projection_body(b))
        .transpose()?
        .ok_or_else(|| internal("missing projection in RETURN", sp))?;
    Ok(ast_c::Return {
        distinct: proj.distinct,
        items: proj.items,
        order: proj.order,
        skip: proj.skip,
        limit: proj.limit,
        span: sp,
    })
}

fn build_foreach(c: ForeachClause) -> Result<ast_c::Foreach> {
    let sp = span_of(c.syntax());
    let variable = c
        .variable()
        .map(|v| build_top_variable(v))
        .ok_or_else(|| internal("missing variable in FOREACH", sp))?;
    let list = c
        .list()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing list in FOREACH", sp))?;
    let updates: Result<Vec<_>> = c
        .clauses()
        .filter_map(|cl| match cl {
            Clause::Create(cc) => Some(build_create(cc).map(ast_c::ForeachUpdate::Create)),
            Clause::Merge(mc) => Some(build_merge(mc).map(ast_c::ForeachUpdate::Merge)),
            Clause::Set(sc) => Some(build_set(sc).map(ast_c::ForeachUpdate::Set)),
            Clause::Remove(rc) => Some(build_remove(rc).map(ast_c::ForeachUpdate::Remove)),
            Clause::Delete(dc) => Some(build_delete(dc).map(ast_c::ForeachUpdate::Delete)),
            Clause::Foreach(fc) => Some(build_foreach(fc).map(ast_c::ForeachUpdate::Foreach)),
            _ => None,
        })
        .collect();
    Ok(ast_c::Foreach {
        variable,
        list,
        updates: updates?,
        span: sp,
    })
}

// ── Projection ───────────────────────────────────────────────────────

struct ProjResult {
    distinct: bool,
    items: Vec<ast_c::ProjectionItem>,
    order: Option<ast_c::Order>,
    skip: Option<ast_c::Expression>,
    limit: Option<ast_c::Expression>,
}

fn build_projection_body(body: ProjectionBody) -> Result<ProjResult> {
    let distinct = body.distinct_token().is_some();
    let items: Result<Vec<_>> = body.items().map(|i| build_projection_item(i)).collect();
    let order = body.order_by().map(|o| build_order(o)).transpose()?;
    let skip = body
        .skip()
        .and_then(|s| s.expr().map(|e| build_expression(e)))
        .transpose()?;
    let limit = body
        .limit()
        .and_then(|l| l.expr().map(|e| build_expression(e)))
        .transpose()?;
    Ok(ProjResult {
        distinct,
        items: items?,
        order,
        skip,
        limit,
    })
}

fn build_projection_item(item: ProjectionItem) -> Result<ast_c::ProjectionItem> {
    let sp = span_of(item.syntax());
    let expr = if let Some(e) = item.expr() {
        build_expression(e)?
    } else if item.syntax().children_with_tokens().any(|t| {
        t.as_token()
            .map_or(false, |t| t.kind() == SyntaxKind::NULL_KW)
    }) {
        ast_c::Expression::Literal(ast_c::Literal::Null)
    } else {
        let int_text = item.syntax().children_with_tokens().find_map(|t| {
            t.as_token()
                .filter(|t| t.kind() == SyntaxKind::INTEGER)
                .map(|t| t.text().to_string())
        });
        if let Some(text) = int_text {
            let val = parse_integer(&text).ok_or_else(|| internal("invalid integer", sp))?;
            ast_c::Expression::Literal(ast_c::Literal::Number(ast_c::NumberLiteral::Integer(val)))
        } else {
            let float_text = item.syntax().children_with_tokens().find_map(|t| {
                t.as_token()
                    .filter(|t| t.kind() == SyntaxKind::FLOAT)
                    .map(|t| t.text().to_string())
            });
            if let Some(text) = float_text {
                let val = parse_double(&text).ok_or_else(|| internal("invalid float", sp))?;
                ast_c::Expression::Literal(ast_c::Literal::Number(ast_c::NumberLiteral::Float(val)))
            } else {
                return Err(internal("missing expr in projection item", sp));
            }
        }
    };
    let alias = item.as_name().map(|v| build_top_variable(v));
    Ok(ast_c::ProjectionItem {
        expression: expr,
        alias,
    })
}

fn build_order(o: OrderBy) -> Result<ast_c::Order> {
    let items: Result<Vec<_>> = o.items().map(|s| build_sort_item(s)).collect();
    Ok(ast_c::Order { items: items? })
}

fn build_sort_item(s: SortItem) -> Result<ast_c::SortItem> {
    let expr = s
        .expr()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing expr in sort item", span_of(s.syntax())))?;
    let direction = s.direction().map(|d| match d {
        SortDirection::Ascending => ast_c::SortDirection::Ascending,
        SortDirection::Descending => ast_c::SortDirection::Descending,
    });
    Ok(ast_c::SortItem {
        expression: expr,
        direction,
    })
}

// ── Patterns ─────────────────────────────────────────────────────────

fn build_pattern(p: Pattern) -> Result<ast_c::Pattern> {
    let sp = span_of(p.syntax());
    let parts: Result<Vec<_>> = p.parts().map(|pp| build_pattern_part(pp)).collect();
    Ok(ast_c::Pattern {
        parts: parts?,
        span: sp,
    })
}

fn build_pattern_part(pp: PatternPart) -> Result<ast_c::PatternPart> {
    let sp = span_of(pp.syntax());
    let variable = pp.variable().map(|v| build_top_variable(v));
    let anonymous = pp
        .anonymous_part()
        .map(|a| build_anonymous_pattern_part(a))
        .transpose()?
        .ok_or_else(|| internal("missing anonymous part", sp))?;
    Ok(ast_c::PatternPart {
        variable,
        anonymous,
        span: sp,
    })
}

fn build_anonymous_pattern_part(app: AnonymousPatternPart) -> Result<ast_c::AnonymousPatternPart> {
    let element = app
        .element()
        .map(|e| build_pattern_element(e))
        .transpose()?
        .ok_or_else(|| internal("missing element", span_of(app.syntax())))?;
    Ok(ast_c::AnonymousPatternPart { element })
}

fn build_pattern_element(pe: PatternElement) -> Result<ast_c::PatternElement> {
    let node = pe.node();
    let chains: Vec<_> = pe.chains().collect();
    if chains.is_empty() {
        if let Some(n) = node {
            return Ok(ast_c::PatternElement::Path {
                start: build_node_pattern(n)?,
                chains: Vec::new(),
            });
        }
    }
    let start = node
        .map(|n| build_node_pattern(n))
        .transpose()?
        .ok_or_else(|| internal("missing node", span_of(pe.syntax())))?;
    let built: Result<Vec<_>> = chains
        .into_iter()
        .map(|c| build_pattern_element_chain(c))
        .collect();
    Ok(ast_c::PatternElement::Path {
        start,
        chains: built?,
    })
}

fn build_node_pattern(np: NodePattern) -> Result<ast_c::NodePattern> {
    let sp = span_of(np.syntax());
    let variable = np.variable().map(|v| build_top_variable(v));
    let labels: Result<Vec<_>> = np
        .labels()
        .flat_map(|container| container.labels())
        .map(|nl| {
            nl.name()
                .and_then(|ln| ln.symbolic_name())
                .map(|s| ast_c::SymbolicName {
                    name: symbolic_name_text(&s),
                    span: span_of(s.syntax()),
                })
                .ok_or_else(|| internal("missing label name", sp))
        })
        .collect();
    let properties = np.properties().map(|p| build_properties(p)).transpose()?;
    Ok(ast_c::NodePattern {
        variable,
        labels: labels?,
        properties,
        span: sp,
    })
}

fn build_pattern_element_chain(pec: PatternElementChain) -> Result<ast_c::PatternElementChain> {
    let sp = span_of(pec.syntax());
    let has_left = pec.syntax().children_with_tokens().any(|t| {
        t.as_token()
            .map_or(false, |t| t.kind() == SyntaxKind::ARROW_LEFT)
    });
    let has_right = pec.syntax().children_with_tokens().any(|t| {
        t.as_token().map_or(false, |t| {
            t.kind() == SyntaxKind::ARROW_RIGHT || t.kind() == SyntaxKind::GT
        })
    });
    let direction = match (has_left, has_right) {
        (true, true) => ast_c::RelationshipDirection::Both,
        (true, false) => ast_c::RelationshipDirection::Left,
        (false, true) => ast_c::RelationshipDirection::Right,
        (false, false) => ast_c::RelationshipDirection::Undirected,
    };
    let detail = pec
        .syntax()
        .children()
        .find_map(|n| RelationshipDetail::cast(n))
        .map(|d| build_relationship_detail(d))
        .transpose()?;
    let relationship = ast_c::RelationshipPattern {
        direction,
        detail,
        span: sp,
    };
    let node = pec
        .node()
        .map(|n| build_node_pattern(n))
        .transpose()?
        .ok_or_else(|| internal("missing node in chain", span_of(pec.syntax())))?;
    Ok(ast_c::PatternElementChain { relationship, node })
}

fn build_relationship_pattern(rp: RelationshipPattern) -> Result<ast_c::RelationshipPattern> {
    let sp = span_of(rp.syntax());
    let direction = if rp.left_arrow().is_some() {
        ast_c::RelationshipDirection::Left
    } else if rp.right_arrow().is_some() {
        ast_c::RelationshipDirection::Right
    } else {
        ast_c::RelationshipDirection::Undirected
    };
    let detail = rp
        .detail()
        .map(|d| build_relationship_detail(d))
        .transpose()?;
    Ok(ast_c::RelationshipPattern {
        direction,
        detail,
        span: sp,
    })
}

fn build_relationship_detail(rd: RelationshipDetail) -> Result<ast_c::RelationshipDetail> {
    let sp = span_of(rd.syntax());
    let variable = rd.variable().map(|v| build_top_variable(v));
    let types: Result<Vec<_>> = rd
        .types()
        .into_iter()
        .flat_map(|container| container.types())
        .map(|t| {
            t.symbolic_name()
                .map(|s| ast_c::RelTypeName {
                    name: ast_c::SymbolicName {
                        name: symbolic_name_text(&s),
                        span: span_of(s.syntax()),
                    },
                })
                .ok_or_else(|| internal("missing rel type", sp))
        })
        .collect();
    let range = rd.range().map(|r| build_range_literal(r)).transpose()?;
    let properties = rd.properties().map(|p| build_properties(p)).transpose()?;
    Ok(ast_c::RelationshipDetail {
        variable,
        types: types?,
        range,
        properties,
        span: sp,
    })
}

fn build_range_literal(rl: RangeLiteral) -> Result<ast_c::RangeLiteral> {
    let sp = span_of(rl.syntax());
    Ok(ast_c::RangeLiteral {
        start: None,
        end: None,
        span: sp,
    })
}

fn build_properties(p: Properties) -> Result<ast_c::Properties> {
    if let Some(map) = p.map_literal() {
        let entries: Result<Vec<_>> = map
            .entries()
            .map(|e| {
                let key = e
                    .key()
                    .map(|k| {
                        k.symbolic_name()
                            .map(|s| ast_c::PropertyKeyName {
                                name: ast_c::SymbolicName {
                                    name: symbolic_name_text(&s),
                                    span: span_of(s.syntax()),
                                },
                            })
                            .ok_or_else(|| internal("missing prop key", span_of(k.syntax())))
                    })
                    .transpose()?
                    .ok_or_else(|| internal("missing key", span_of(e.syntax())))?;
                let value = e
                    .value()
                    .map(|v| build_expression(v))
                    .transpose()?
                    .ok_or_else(|| internal("missing value", span_of(e.syntax())))?;
                Ok((key, value))
            })
            .collect();
        Ok(ast_c::Properties::Map(ast_c::MapLiteral {
            entries: entries?,
            span: span_of(map.syntax()),
        }))
    } else {
        Err(internal(
            "unsupported properties shape",
            span_of(p.syntax()),
        ))
    }
}

// ── Expressions ──────────────────────────────────────────────────────

fn build_expression(e: Expression) -> Result<ast_c::Expression> {
    match e {
        Expression::BinaryExpr(b) => build_binary_expr(b),
        Expression::UnaryExpr(u) => build_unary_expr(u),
        Expression::Atom(a) => build_atom(a),
    }
}

fn build_binary_expr(b: BinaryExpr) -> Result<ast_c::Expression> {
    let sp = span_of(b.syntax());
    let lhs = b
        .lhs()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing lhs", sp))?;

    match b.op_kind() {
        Some(BinOp::IsNull) => Ok(ast_c::Expression::IsNull {
            operand: Box::new(lhs),
            negated: false,
            span: sp,
        }),
        Some(BinOp::IsNotNull) => Ok(ast_c::Expression::IsNull {
            operand: Box::new(lhs),
            negated: true,
            span: sp,
        }),
        _ => {
            let rhs = b
                .rhs()
                .map(|e| build_expression(e))
                .transpose()?
                .ok_or_else(|| internal("missing rhs", sp))?;

            match b.op_kind() {
                Some(BinOp::Or) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Or,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Xor) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Xor,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::And) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Eq) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Eq, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Ne) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Ne, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Lt) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Lt, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Gt) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Gt, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Le) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Le, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Ge) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Ge, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Add) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Add,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Sub) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Subtract,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Mul) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Multiply,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Div) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Divide,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Mod) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Modulo,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Power) => Ok(ast_c::Expression::BinaryOp {
                    op: ast_c::BinaryOperator::Power,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::StartsWith) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::StartsWith, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::EndsWith) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::EndsWith, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::Contains) => Ok(ast_c::Expression::Comparison {
                    lhs: Box::new(lhs),
                    operators: vec![(ast_c::ComparisonOperator::Contains, Box::new(rhs))],
                    span: sp,
                }),
                Some(BinOp::In) => Ok(ast_c::Expression::In {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::Index) => Ok(ast_c::Expression::ListIndex {
                    list: Box::new(lhs),
                    index: Box::new(rhs),
                    span: sp,
                }),
                Some(BinOp::PropertyLookup) => Ok(ast_c::Expression::PropertyLookup {
                    base: Box::new(lhs),
                    property: extract_property_key(&rhs)?,
                    span: sp,
                }),
                Some(BinOp::HasLabel) => {
                    let labels = extract_labels(&rhs)?;
                    Ok(ast_c::Expression::NodeLabels {
                        base: Box::new(lhs),
                        labels,
                        span: sp,
                    })
                }
                None => Err(internal("unknown binary op", sp)),
                _ => Err(internal("unexpected binary op", sp)),
            }
        }
    }
}

fn extract_property_key(e: &ast_c::Expression) -> Result<ast_c::PropertyKeyName> {
    if let ast_c::Expression::Literal(ast_c::Literal::String(s)) = e {
        Ok(ast_c::PropertyKeyName {
            name: ast_c::SymbolicName {
                name: s.value.clone(),
                span: s.span,
            },
        })
    } else if let ast_c::Expression::Variable(v) = e {
        Ok(ast_c::PropertyKeyName {
            name: v.name.clone(),
        })
    } else {
        Err(internal("expected property key", Span::new(0, 0)))
    }
}

fn extract_labels(_e: &ast_c::Expression) -> Result<Vec<ast_c::SymbolicName>> {
    Err(internal(
        "label extraction from binary expr not implemented",
        Span::new(0, 0),
    ))
}

fn build_unary_expr(u: UnaryExpr) -> Result<ast_c::Expression> {
    let sp = span_of(u.syntax());
    let operand = u
        .operand()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing operand", sp))?;
    match u.op() {
        Some(UnOp::Not) => Ok(ast_c::Expression::UnaryOp {
            op: ast_c::UnaryOperator::Not,
            operand: Box::new(operand),
            span: sp,
        }),
        Some(UnOp::Neg) => Ok(ast_c::Expression::UnaryOp {
            op: ast_c::UnaryOperator::Negate,
            operand: Box::new(operand),
            span: sp,
        }),
        Some(UnOp::Pos) => Ok(ast_c::Expression::UnaryOp {
            op: ast_c::UnaryOperator::Plus,
            operand: Box::new(operand),
            span: sp,
        }),
        None => Err(internal("unknown unary op", sp)),
    }
}

fn build_atom(a: Atom) -> Result<ast_c::Expression> {
    match a {
        Atom::Literal(l) => build_literal(l),
        Atom::Variable(v) => Ok(ast_c::Expression::Variable(build_variable(v))),
        Atom::Parameter(p) => build_parameter(p),
        Atom::FunctionInvocation(f) => build_function_invocation(f),
        Atom::Parenthesized(pe) => build_parenthesized(pe),
        Atom::Case(c) => build_case(c),
        Atom::ListLiteral(ll) => build_list_literal(ll),
        Atom::MapLiteral(ml) => build_map_literal(ml),
        Atom::ListComprehension(lc) => build_list_comprehension(lc),
        Atom::PatternComprehension(pc) => build_pattern_comprehension(pc),
        Atom::FilterExpression(fe) => build_filter_expression(fe),
        Atom::ExistsSubquery(es) => build_exists_subquery(es),
        Atom::MapProjection(mp) => build_map_projection(mp),
        Atom::ImplicitProcedureInvocation(ipi) => {
            let proc = build_implicit_procedure_invocation(ipi)?;
            Ok(ast_c::Expression::FunctionCall(proc.name))
        }
    }
}

fn build_literal(l: Literal) -> Result<ast_c::Expression> {
    match l {
        Literal::Number(n) => {
            let num = build_number_literal(n)?;
            Ok(ast_c::Expression::Literal(ast_c::Literal::Number(num)))
        }
        Literal::String(s) => {
            let sl = build_string_literal(s)?;
            Ok(ast_c::Expression::Literal(ast_c::Literal::String(sl)))
        }
        Literal::Boolean(b) => Ok(ast_c::Expression::Literal(ast_c::Literal::Boolean(
            b.value(),
        ))),
        Literal::Null(_n) => Ok(ast_c::Expression::Literal(ast_c::Literal::Null)),
    }
}

fn build_number_literal(n: NumberLiteral) -> Result<ast_c::NumberLiteral> {
    let sp = span_of(n.syntax());
    if let Some(tok) = n.token() {
        let text = tok.text();
        if tok.kind() == SyntaxKind::INTEGER {
            if let Some(val) = parse_integer(text) {
                return Ok(ast_c::NumberLiteral::Integer(val));
            }
        } else if tok.kind() == SyntaxKind::FLOAT {
            if let Some(val) = parse_double(text) {
                return Ok(ast_c::NumberLiteral::Float(val));
            }
        }
    }
    Err(internal("invalid number", sp))
}

fn build_string_literal(s: StringLiteral) -> Result<ast_c::StringLiteral> {
    let sp = span_of(s.syntax());
    if let Some(tok) = s.token() {
        let raw = tok.text();
        let content = if (raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\''))
        {
            &raw[1..raw.len() - 1]
        } else {
            raw
        };
        let (value, err) = decode_string_content(content, sp);
        if let Some(e) = err {
            return Err(e);
        }
        return Ok(ast_c::StringLiteral {
            value,
            span: sp,
            raw: Some(raw.to_string()),
        });
    }
    Err(internal("missing string token", sp))
}

fn build_parameter(_p: expressions::Parameter) -> Result<ast_c::Expression> {
    // TODO: extract parameter name from CST
    Err(internal(
        "parameter parsing not yet implemented",
        Span::new(0, 0),
    ))
}

fn build_function_invocation(f: FunctionInvocation) -> Result<ast_c::Expression> {
    let sp = span_of(f.syntax());
    let name_parts: Vec<ast_c::SymbolicName> = f
        .name()
        .into_iter()
        .flat_map(|fn_name| {
            fn_name
                .symbolic_names()
                .map(|s| ast_c::SymbolicName {
                    name: symbolic_name_text(&s),
                    span: span_of(s.syntax()),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let distinct = f.distinct_token().is_some();
    let star = f.star_token().is_some();
    let args: Result<Vec<_>> = f.arguments().map(|e| build_expression(e)).collect();
    let args = args?;

    if star
        && args.is_empty()
        && name_parts.len() == 1
        && name_parts[0].name.to_lowercase() == "count"
    {
        return Ok(ast_c::Expression::CountStar { span: sp });
    }

    Ok(ast_c::Expression::FunctionCall(ast_c::FunctionInvocation {
        name: name_parts,
        distinct,
        arguments: args,
        span: sp,
    }))
}

fn build_parenthesized(pe: ParenthesizedExpr) -> Result<ast_c::Expression> {
    let inner = pe
        .expr()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing expr in parens", span_of(pe.syntax())))?;
    Ok(ast_c::Expression::Parenthesized(Box::new(inner)))
}

fn build_case(c: CaseExpr) -> Result<ast_c::Expression> {
    let sp = span_of(c.syntax());
    let scrutinee = c.value().map(|e| build_expression(e)).transpose()?;
    let alts: Result<Vec<_>> = c
        .alternatives()
        .map(|a| {
            let when = a
                .when_expr()
                .map(|e| build_expression(e))
                .transpose()?
                .ok_or_else(|| internal("missing when", span_of(a.syntax())))?;
            let then = a
                .then_expr()
                .map(|e| build_expression(e))
                .transpose()?
                .ok_or_else(|| internal("missing then", span_of(a.syntax())))?;
            Ok(ast_c::CaseAlternative { when, then })
        })
        .collect();
    let default = c.else_expr().map(|e| build_expression(e)).transpose()?;
    Ok(ast_c::Expression::Case(ast_c::CaseExpression {
        scrutinee: scrutinee.map(Box::new),
        alternatives: alts?,
        default: default.map(Box::new),
        span: sp,
    }))
}

fn build_list_literal(ll: ListLiteral) -> Result<ast_c::Expression> {
    let sp = span_of(ll.syntax());
    let elems: Result<Vec<_>> = ll.elements().map(|e| build_expression(e)).collect();
    Ok(ast_c::Expression::Literal(ast_c::Literal::List(
        ast_c::ListLiteral {
            elements: elems?,
            span: sp,
        },
    )))
}

fn build_map_literal(ml: MapLiteral) -> Result<ast_c::Expression> {
    let sp = span_of(ml.syntax());
    let entries: Result<Vec<_>> = ml
        .entries()
        .map(|e| {
            let key = e
                .key()
                .map(|k| {
                    k.symbolic_name()
                        .map(|s| ast_c::PropertyKeyName {
                            name: ast_c::SymbolicName {
                                name: symbolic_name_text(&s),
                                span: span_of(s.syntax()),
                            },
                        })
                        .ok_or_else(|| internal("missing prop key", span_of(k.syntax())))
                })
                .transpose()?
                .ok_or_else(|| internal("missing key", span_of(e.syntax())))?;
            let value = e
                .value()
                .map(|v| build_expression(v))
                .transpose()?
                .ok_or_else(|| internal("missing value", span_of(e.syntax())))?;
            Ok((key, value))
        })
        .collect();
    Ok(ast_c::Expression::Literal(ast_c::Literal::Map(
        ast_c::MapLiteral {
            entries: entries?,
            span: sp,
        },
    )))
}

fn build_list_comprehension(lc: ListComprehension) -> Result<ast_c::Expression> {
    let sp = span_of(lc.syntax());
    if let Some(filter) = lc.filter() {
        let var = filter
            .id_in_coll()
            .and_then(|id| id.variable())
            .map(|v| build_variable(v))
            .ok_or_else(|| internal("missing variable in list comp", sp))?;
        let coll = filter
            .id_in_coll()
            .and_then(|id| id.collection())
            .map(|e| build_expression(e))
            .ok_or_else(|| internal("missing collection in list comp", sp))?;
        let pred = filter
            .where_clause()
            .and_then(|w| w.expr())
            .map(|e| build_expression(e))
            .transpose()?;
        let map = lc.body().map(|e| build_expression(e)).transpose()?;
        Ok(ast_c::Expression::ListComprehension(Box::new(
            ast_c::ListComprehension {
                variable: var,
                filter: pred.map(Box::new),
                map,
                span: sp,
            },
        )))
    } else {
        Err(internal("missing filter in list comp", sp))
    }
}

fn build_pattern_comprehension(pc: PatternComprehension) -> Result<ast_c::Expression> {
    let sp = span_of(pc.syntax());
    let variable = pc.variable().map(|v| build_variable(v));
    let _pat = pc.pattern();
    let where_clause = pc
        .where_clause()
        .and_then(|w| w.expr())
        .map(|e| build_expression(e))
        .transpose()?;
    let map = pc
        .body()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing body in pattern comp", sp))?;
    let placeholder = ast_c::RelationshipsPattern {
        start: ast_c::NodePattern {
            variable: variable.clone(),
            labels: Vec::new(),
            properties: None,
            span: sp,
        },
        chains: Vec::new(),
        span: sp,
    };
    Ok(ast_c::Expression::PatternComprehension(Box::new(
        ast_c::PatternComprehension {
            variable,
            pattern: placeholder,
            where_clause,
            map,
            span: sp,
        },
    )))
}

fn build_filter_expression(fe: FilterExpression) -> Result<ast_c::Expression> {
    let sp = span_of(fe.syntax());
    let id = fe
        .id_in_coll()
        .ok_or_else(|| internal("missing IdInColl", sp))?;
    let var = id
        .variable()
        .map(|v| build_variable(v))
        .ok_or_else(|| internal("missing variable", sp))?;
    let coll = id
        .collection()
        .map(|e| build_expression(e))
        .transpose()?
        .ok_or_else(|| internal("missing collection", sp))?;
    let pred = fe
        .where_clause()
        .and_then(|w| w.expr())
        .map(|e| build_expression(e))
        .transpose()?;
    Ok(ast_c::Expression::Any(Box::new(ast_c::FilterExpression {
        variable: var,
        collection: Box::new(coll),
        predicate: pred.map(Box::new),
        span: sp,
    })))
}

fn build_exists_subquery(es: ExistsSubquery) -> Result<ast_c::Expression> {
    let sp = span_of(es.syntax());
    let _pat = es.pattern();
    let where_clause = es
        .where_clause()
        .and_then(|w| w.expr())
        .map(|e| build_expression(e))
        .transpose()?;
    let placeholder = ast_c::Pattern {
        parts: Vec::new(),
        span: sp,
    };
    Ok(ast_c::Expression::Exists(Box::new(
        ast_c::ExistsExpression {
            inner: Box::new(ast_c::ExistsInner::Pattern(
                placeholder,
                where_clause.map(Box::new),
            )),
            span: sp,
        },
    )))
}

fn build_map_projection(mp: MapProjection) -> Result<ast_c::Expression> {
    let sp = span_of(mp.syntax());
    let base = mp
        .variable()
        .map(|v| build_variable(v))
        .ok_or_else(|| internal("missing base in map proj", sp))?;
    let items: Result<Vec<_>> = mp.items().map(|i| build_map_projection_item(i)).collect();
    Ok(ast_c::Expression::MapProjection(Box::new(
        ast_c::MapProjection {
            base,
            items: items?,
            span: sp,
        },
    )))
}

fn build_map_projection_item(mi: MapProjectionItem) -> Result<ast_c::MapProjectionItem> {
    let sp = span_of(mi.syntax());
    if mi.is_star() {
        return Ok(ast_c::MapProjectionItem::AllProperties { span: sp });
    }
    if let Some(key) = mi.property_name() {
        let pk = ast_c::PropertyKeyName {
            name: key
                .symbolic_name()
                .map(|s| ast_c::SymbolicName {
                    name: symbolic_name_text(&s),
                    span: span_of(s.syntax()),
                })
                .ok_or_else(|| internal("missing prop key", sp))?,
        };
        if let Some(expr) = mi.expression() {
            let expr_ast = build_expression(expr)?;
            if let ast_c::Expression::Variable(v) = &expr_ast {
                if v.name.name == pk.name.name {
                    return Ok(ast_c::MapProjectionItem::PropertyLookup { property: pk });
                }
            }
            return Ok(ast_c::MapProjectionItem::Literal {
                key: pk,
                value: expr_ast,
            });
        }
        return Ok(ast_c::MapProjectionItem::PropertyLookup { property: pk });
    }
    Err(internal("unrecognized map proj item", sp))
}

// ── Procedure calls ──────────────────────────────────────────────────

fn build_standalone_call(c: StandaloneCall) -> Result<ast_c::StandaloneCall> {
    let sp = span_of(c.syntax());
    let call = if let Some(exp) = c.explicit_invocation() {
        build_explicit_procedure_invocation(exp)?
    } else if let Some(imp) = c.implicit_invocation() {
        build_implicit_procedure_invocation(imp)?
    } else {
        return Err(internal("missing proc invocation", sp));
    };
    let yield_items = c
        .yield_items()
        .map(|y| {
            if y.star_token().is_some() {
                Ok(ast_c::YieldSpec::Star {
                    span: span_of(y.syntax()),
                })
            } else {
                let items: Result<Vec<_>> = y.items().map(|i| build_yield_item(i)).collect();
                let wc = y.where_expr().map(|e| build_expression(e)).transpose()?;
                Ok(ast_c::YieldSpec::Items(ast_c::YieldItems {
                    items: items?,
                    where_clause: wc,
                }))
            }
        })
        .transpose()?;
    Ok(ast_c::StandaloneCall {
        call,
        yield_items,
        span: sp,
    })
}

fn build_explicit_procedure_invocation(
    e: ExplicitProcedureInvocation,
) -> Result<ast_c::ProcedureInvocation> {
    let sp = span_of(e.syntax());
    let name = if let Some(pn) = e.procedure_name() {
        build_procedure_name(pn)?
    } else {
        return Err(internal("missing proc name", sp));
    };
    let _args: Result<Vec<_>> = e.arguments().map(|a| build_expression(a)).collect();
    Ok(ast_c::ProcedureInvocation { name, span: sp })
}

fn build_implicit_procedure_invocation(
    i: ImplicitProcedureInvocation,
) -> Result<ast_c::ProcedureInvocation> {
    let sp = span_of(i.syntax());
    let name = if let Some(pn) = i.procedure_name() {
        build_procedure_name(pn)?
    } else {
        return Err(internal("missing proc name", sp));
    };
    Ok(ast_c::ProcedureInvocation { name, span: sp })
}

fn build_procedure_name(pn: ProcedureName) -> Result<ast_c::FunctionInvocation> {
    let sp = span_of(pn.syntax());
    let names: Vec<_> = pn
        .symbolic_names()
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .collect();
    let (ns, pn_name) = if names.is_empty() {
        (
            Vec::new(),
            ast_c::SymbolicName {
                name: String::new(),
                span: sp,
            },
        )
    } else {
        let last = names.last().unwrap().clone();
        (names[..names.len() - 1].to_vec(), last)
    };
    let full: Vec<_> = ns.into_iter().chain(std::iter::once(pn_name)).collect();
    Ok(ast_c::FunctionInvocation {
        name: full,
        distinct: false,
        arguments: Vec::new(),
        span: sp,
    })
}

fn build_in_query_call(c: InQueryCall) -> Result<ast_c::InQueryCall> {
    let sp = span_of(c.syntax());
    let yield_items = c
        .yield_items()
        .map(|y| {
            let items: Result<Vec<_>> = y.items().map(|i| build_yield_item(i)).collect();
            let wc = y.where_expr().map(|e| build_expression(e)).transpose()?;
            Ok(ast_c::YieldItems {
                items: items?,
                where_clause: wc,
            })
        })
        .transpose()?;
    Ok(ast_c::InQueryCall {
        call: ast_c::ProcedureInvocation {
            name: ast_c::FunctionInvocation {
                name: Vec::new(),
                distinct: false,
                arguments: Vec::new(),
                span: sp,
            },
            span: sp,
        },
        yield_items,
        span: sp,
    })
}

fn build_yield_item(yi: YieldItem) -> Result<ast_c::YieldItem> {
    let pf = yi
        .field_name()
        .and_then(|f| f.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .ok_or_else(|| internal("missing proc field", span_of(yi.syntax())))?;
    let alias = yi.alias().map(|v| build_top_variable(v));
    Ok(ast_c::YieldItem {
        procedure_field: pf,
        alias,
    })
}

fn build_call_subquery(c: CallSubqueryClause) -> Result<ast_c::CallSubquery> {
    let sp = span_of(c.syntax());
    let inner_clauses: Vec<_> = c.inner_clauses().collect();
    let inner_unions: Vec<_> = c.inner_unions().collect();
    let query = if inner_unions.is_empty() {
        let single = build_single_query_from_clauses(inner_clauses)?;
        ast_c::RegularQuery {
            single_query: single,
            unions: Vec::new(),
        }
    } else {
        build_regular_query(&inner_clauses, &inner_unions)?
    };
    let in_tx = c
        .in_transactions()
        .map(|it| build_in_transactions(it))
        .transpose()?;
    Ok(ast_c::CallSubquery {
        query,
        in_transactions: in_tx,
        span: sp,
    })
}

fn build_in_transactions(it: InTransactions) -> Result<ast_c::InTransactions> {
    let sp = span_of(it.syntax());
    let of_rows = it.rows_expr().and_then(|n| {
        let text = n.token().map(|t| t.text().to_string());
        text.and_then(|t| parse_integer(&t)).map(|v| {
            ast_c::Expression::Literal(ast_c::Literal::Number(ast_c::NumberLiteral::Integer(v)))
        })
    });
    let on_error = it.on_error_action().and_then(|tok| match tok.kind() {
        SyntaxKind::KW_CONTINUE => Some(ast_c::OnErrorBehavior::Continue),
        SyntaxKind::KW_BREAK => Some(ast_c::OnErrorBehavior::Break),
        SyntaxKind::KW_FAIL => Some(ast_c::OnErrorBehavior::Fail),
        _ => None,
    });
    Ok(ast_c::InTransactions {
        of_rows,
        on_error,
        span: sp,
    })
}

// ── Schema commands ──────────────────────────────────────────────────

fn build_schema_command(cmd: cst_c::schema_cst::SchemaCommand) -> Result<ast_c::SchemaCommand> {
    match cmd {
        cst_c::schema_cst::SchemaCommand::CreateIndex(c) => {
            Ok(ast_c::SchemaCommand::CreateIndex(build_create_index(c)?))
        }
        cst_c::schema_cst::SchemaCommand::DropIndex(d) => {
            Ok(ast_c::SchemaCommand::DropIndex(build_drop_index(d)?))
        }
        cst_c::schema_cst::SchemaCommand::CreateConstraint(c) => Ok(
            ast_c::SchemaCommand::CreateConstraint(build_create_constraint(c)?),
        ),
        cst_c::schema_cst::SchemaCommand::DropConstraint(d) => Ok(
            ast_c::SchemaCommand::DropConstraint(build_drop_constraint(d)?),
        ),
    }
}

fn build_create_index(c: cst_c::schema_cst::CreateIndex) -> Result<ast_c::CreateIndex> {
    let sp = span_of(c.syntax());
    let if_not_exists = c.if_not_exists();
    let name = c
        .name()
        .and_then(|n| n.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        });
    let options = c
        .options()
        .and_then(|o| o.map())
        .map(|m| build_map_literal_cst(m))
        .transpose()?;
    let target = c
        .label()
        .and_then(|l| l.variable())
        .map(|v| build_top_variable(v))
        .map(|v| v.name)
        .ok_or_else(|| internal("missing index target", sp))?;
    Ok(ast_c::CreateIndex {
        kind: None,
        if_not_exists,
        name,
        target,
        options,
        span: sp,
    })
}

fn build_map_literal_cst(m: MapLiteral) -> Result<ast_c::MapLiteral> {
    let sp = span_of(m.syntax());
    let entries: Result<Vec<_>> = m
        .entries()
        .map(|e| {
            let key = e
                .key()
                .map(|k| {
                    k.symbolic_name()
                        .map(|s| ast_c::PropertyKeyName {
                            name: ast_c::SymbolicName {
                                name: symbolic_name_text(&s),
                                span: span_of(s.syntax()),
                            },
                        })
                        .ok_or_else(|| internal("missing prop key", span_of(k.syntax())))
                })
                .transpose()?
                .ok_or_else(|| internal("missing key", span_of(e.syntax())))?;
            let value = e
                .value()
                .map(|v| build_expression(v))
                .transpose()?
                .ok_or_else(|| internal("missing value", span_of(e.syntax())))?;
            Ok((key, value))
        })
        .collect();
    Ok(ast_c::MapLiteral {
        entries: entries?,
        span: sp,
    })
}

fn build_drop_index(d: cst_c::schema_cst::DropIndex) -> Result<ast_c::DropIndex> {
    let sp = span_of(d.syntax());
    let if_exists = d.if_exists();
    let name = d
        .name()
        .and_then(|n| n.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .ok_or_else(|| internal("missing index name", sp))?;
    Ok(ast_c::DropIndex {
        if_exists,
        name,
        span: sp,
    })
}

fn build_create_constraint(
    c: cst_c::schema_cst::CreateConstraint,
) -> Result<ast_c::CreateConstraint> {
    let sp = span_of(c.syntax());
    let name = c
        .name()
        .and_then(|n| n.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        });
    let variable = c
        .options()
        .and_then(|o| o.map())
        .and_then(|m| m.entries().next())
        .map(|e| build_variable_from_entry(e))
        .ok_or_else(|| internal("missing constraint variable", sp))?;
    let kind = ast_c::ConstraintKind::Unique;
    Ok(ast_c::CreateConstraint {
        name,
        variable,
        kind,
        span: sp,
    })
}

fn build_variable_from_entry(e: MapEntry) -> ast_c::Variable {
    let name = e
        .key()
        .and_then(|k| k.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .unwrap_or_else(|| ast_c::SymbolicName {
            name: String::new(),
            span: Span::new(0, 0),
        });
    ast_c::Variable { name }
}

fn build_drop_constraint(d: cst_c::schema_cst::DropConstraint) -> Result<ast_c::DropConstraint> {
    let sp = span_of(d.syntax());
    let if_exists = d.if_exists();
    let name = d
        .name()
        .and_then(|n| n.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .ok_or_else(|| internal("missing constraint name", sp))?;
    Ok(ast_c::DropConstraint {
        if_exists,
        name,
        span: sp,
    })
}

fn build_show(c: ShowClause) -> Result<ast_c::Show> {
    let sp = span_of(c.syntax());
    let kind = c
        .kind()
        .map(|k| build_show_kind(k))
        .transpose()?
        .ok_or_else(|| internal("missing SHOW kind", sp))?;
    let yield_items = c
        .show_return()
        .map(|sr| {
            if sr.star_token().is_some() {
                Ok(ast_c::ShowYieldSpec::Star {
                    span: span_of(sr.syntax()),
                })
            } else {
                let items: Result<Vec<_>> = sr
                    .yield_items()
                    .map(|yi| {
                        let pf = yi
                            .field_name()
                            .and_then(|f| f.symbolic_name())
                            .map(|s| ast_c::SymbolicName {
                                name: symbolic_name_text(&s),
                                span: span_of(s.syntax()),
                            })
                            .ok_or_else(|| internal("missing proc field", span_of(yi.syntax())))?;
                        let alias = yi.alias().map(|v| build_top_variable(v));
                        Ok(ast_c::ShowYieldItem {
                            procedure_field: pf,
                            alias,
                        })
                    })
                    .collect();
                Ok(ast_c::ShowYieldSpec::Items(items?))
            }
        })
        .transpose()?;
    let where_clause = c
        .show_return()
        .and_then(|sr| sr.where_expr())
        .map(|e| build_expression(e))
        .transpose()?;
    let ret_clause = c
        .return_clause()
        .map(|rc| build_return_body(rc))
        .transpose()?;
    Ok(ast_c::Show {
        kind,
        yield_items,
        where_clause,
        return_clause: ret_clause,
        span: sp,
    })
}

fn build_show_kind(k: ShowKind) -> Result<ast_c::ShowKind> {
    let text = k.syntax().text().to_string().to_uppercase();
    if text.contains("INDEX") {
        Ok(ast_c::ShowKind::Indexes)
    } else if text.contains("CONSTRAINT") {
        Ok(ast_c::ShowKind::Constraints)
    } else if text.contains("FUNCTION") {
        Ok(ast_c::ShowKind::Functions)
    } else if text.contains("PROCEDURE") {
        Ok(ast_c::ShowKind::Procedures)
    } else if text.contains("DATABASE") {
        if text == "DATABASES" || text.ends_with("ES") {
            Ok(ast_c::ShowKind::Databases)
        } else {
            let name = k
                .syntax()
                .children()
                .filter_map(|n| top_level::SymbolicName::cast(n))
                .next()
                .map(|s| ast_c::SymbolicName {
                    name: symbolic_name_text(&s),
                    span: span_of(s.syntax()),
                })
                .ok_or_else(|| internal("missing db name", span_of(k.syntax())))?;
            Ok(ast_c::ShowKind::Database(name))
        }
    } else {
        Ok(ast_c::ShowKind::Indexes)
    }
}

fn build_return_body(rc: ReturnClause) -> Result<ast_c::ReturnBody> {
    let proj = rc
        .projection_body()
        .map(|b| build_projection_body(b))
        .transpose()?
        .ok_or_else(|| internal("missing projection", span_of(rc.syntax())))?;
    Ok(ast_c::ReturnBody {
        distinct: proj.distinct,
        items: proj.items,
        order: proj.order,
        skip: proj.skip,
        limit: proj.limit,
    })
}

fn build_use(c: UseClause) -> Result<ast_c::Use> {
    let sp = span_of(c.syntax());
    let name = c
        .schema_name()
        .and_then(|sn| sn.symbolic_name())
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .ok_or_else(|| internal("missing graph name in USE", sp))?;
    Ok(ast_c::Use {
        graph: name,
        span: sp,
    })
}

// ── Names ────────────────────────────────────────────────────────────

fn build_variable(v: expressions::Variable) -> ast_c::Variable {
    let name = v
        .name()
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .unwrap_or_else(|| ast_c::SymbolicName {
            name: String::new(),
            span: span_of(v.syntax()),
        });
    ast_c::Variable { name }
}

fn build_top_variable(v: top_level::Variable) -> ast_c::Variable {
    let name = v
        .name()
        .map(|s| ast_c::SymbolicName {
            name: symbolic_name_text(&s),
            span: span_of(s.syntax()),
        })
        .unwrap_or_else(|| ast_c::SymbolicName {
            name: String::new(),
            span: span_of(v.syntax()),
        });
    ast_c::Variable { name }
}
