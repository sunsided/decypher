//! Aggregation rule validation.
//!
//! In a `WITH` or `RETURN` that aggregates, every item without an aggregate
//! is a grouping key. An item that contains an aggregate may only use, outside
//! its aggregate calls, constants, parameters, variables local to the item
//! (`[x IN collect(n) | x.a]`) and grouping keys. Only a variable or a
//! property access (`n`, `n.a`) is recognised as a grouping key, so
//! `RETURN n.a, n.a + count(*)` and `RETURN n, n.a + count(*)` are valid while
//! `RETURN n.a + count(*)` and `RETURN n.a + n.b, n.a + n.b + count(*)` are
//! not: they group by `n.a` / `n.b` implicitly.
//!
//! The entry points are [`check_aggregation`] and [`check_aggregation_with`].

use crate::ast::ToCypher;
use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::names::Variable;
use crate::ast::query::*;
use crate::ast::visit::{Visit, walk_expression};
use crate::error::{Diagnostics, Span};
use crate::sema::AggregateRegistry;
use crate::sema::error::SemaError;
use std::collections::HashSet;

/// A marker type for an aggregation-rule violation.
///
/// Currently unused as a concrete value; violations are reported directly
/// through the [`Diagnostics`] collector.
pub struct AggregationViolation;

/// Check aggregation rules for all statements in `query`, treating the
/// built-in aggregate functions as aggregates.
///
/// Appends any violations found to `diagnostics`.
pub fn check_aggregation(query: &Query, diagnostics: &mut Diagnostics) {
    check_aggregation_with(query, &AggregateRegistry::default(), diagnostics);
}

/// Check aggregation rules for all statements in `query`, treating the
/// functions in `aggregates` as aggregates.
///
/// Appends any violations found to `diagnostics`.
///
/// # Example
///
/// ```
/// use decypher::sema::{AggregateRegistry, check_aggregation_with};
/// use decypher::{Diagnostics, parse};
///
/// let query = parse("MATCH (n) RETURN n.a + apoc.agg.median(n.b)").unwrap();
/// let mut aggregates = AggregateRegistry::default();
/// aggregates.register("apoc.agg.median");
/// let mut diagnostics = Diagnostics { errors: Vec::new() };
/// check_aggregation_with(&query, &aggregates, &mut diagnostics);
/// // `n.a` is used next to an aggregate without being a grouping key.
/// assert_eq!(diagnostics.len(), 1);
/// ```
pub fn check_aggregation_with(
    query: &Query,
    aggregates: &AggregateRegistry,
    diagnostics: &mut Diagnostics,
) {
    let is_aggregate = |fc: &FunctionInvocation| aggregates.is_aggregate_call(fc);
    for (function, span) in misplaced_aggregates(query, &is_aggregate) {
        diagnostics
            .errors
            .push(SemaError::AggregateNotAllowed { function, span }.into_error());
    }
    let mut checker = AggregationChecker {
        aggregates,
        diagnostics,
    };
    for stmt in &query.statements {
        match stmt {
            QueryBody::SingleQuery(sq) => checker.check_single_query(sq),
            QueryBody::Regular(rq) => {
                checker.check_single_query(&rq.single_query);
                for union in &rq.unions {
                    checker.check_single_query(&union.single_query);
                }
            }
            QueryBody::Standalone(_)
            | QueryBody::SchemaCommand(_)
            | QueryBody::Show(_)
            | QueryBody::Use(_) => {}
        }
    }
}

/// Internal checker that walks the query and emits aggregation diagnostics.
struct AggregationChecker<'a> {
    /// The functions treated as aggregates.
    aggregates: &'a AggregateRegistry,
    /// The diagnostics collector to which violations are appended.
    diagnostics: &'a mut Diagnostics,
}

impl AggregationChecker<'_> {
    /// Dispatch to the appropriate checker for a single or multi-part query.
    fn check_single_query(&mut self, sq: &SingleQuery) {
        match &sq.kind {
            SingleQueryKind::SinglePart(sp) => self.check_body(&sp.body),
            SingleQueryKind::MultiPart(mp) => {
                for part in &mp.parts {
                    self.check_projection(&part.with.items, part.with.star, part.with.span);
                }
                self.check_body(&mp.final_part.body);
            }
        }
    }

    /// Check the `RETURN` ending a query, if any.
    fn check_body(&mut self, body: &SinglePartBody) {
        let ret = match body {
            SinglePartBody::Return(ret) => Some(ret),
            SinglePartBody::Updating { return_clause, .. } => return_clause.as_ref(),
            SinglePartBody::Finish(_) => None,
        };
        if let Some(ret) = ret {
            self.check_projection(&ret.items, ret.star, ret.span);
        }
    }

    /// Report the implicit grouping keys of a `WITH` or `RETURN` item list.
    fn check_projection(&mut self, items: &[ProjectionItem], star: bool, span: Span) {
        let is_aggregate = |fc: &FunctionInvocation| self.aggregates.is_aggregate_call(fc);
        let non_grouping: Vec<String> = implicit_grouping_keys(items, star, &is_aggregate)
            .into_iter()
            .flat_map(|(_, names)| names)
            .collect();
        if !non_grouping.is_empty() {
            self.diagnostics
                .errors
                .push(SemaError::AggregationMix { non_grouping, span }.into_error());
        }
    }
}

/// For each item of a projection that contains an aggregate, the references
/// it makes outside its aggregate calls that are not grouping keys, with the
/// item's span. Items without such references are left out.
///
/// `RETURN n.a + count(*)` yields `n.a`; `RETURN n, n.a + count(*)` nothing.
/// With `star` (`WITH *` / `RETURN *`) every variable in scope is projected,
/// so every variable is a grouping key.
pub(crate) fn implicit_grouping_keys(
    items: &[ProjectionItem],
    star: bool,
    is_aggregate: &dyn Fn(&FunctionInvocation) -> bool,
) -> Vec<(Span, Vec<String>)> {
    let (aggregating, keys): (Vec<_>, Vec<_>) = items
        .iter()
        .partition(|item| item.expression.contains_aggregate(is_aggregate));
    if aggregating.is_empty() {
        return Vec::new();
    }
    let keys: HashSet<String> = keys
        .iter()
        .map(|item| unparenthesized(&item.expression))
        .filter(|expr| is_reference(expr))
        .map(|expr| expr.to_cypher())
        .collect();

    aggregating
        .into_iter()
        .filter_map(|item| {
            let mut finder = UngroupedReferences {
                is_aggregate,
                star,
                keys: &keys,
                locals: Vec::new(),
                found: Vec::new(),
            };
            finder.visit_expression(&item.expression);
            (!finder.found.is_empty()).then(|| (item.expression.span(), finder.found))
        })
        .collect()
}

fn unparenthesized(mut expr: &Expression) -> &Expression {
    while let Expression::Parenthesized(inner) = expr {
        expr = inner;
    }
    expr
}

/// Whether `expr` is a variable or a chain of property accesses on one
/// (`n`, `n.a`, `m.k.x`), the only expressions recognised as grouping keys.
fn is_reference(expr: &Expression) -> bool {
    match unparenthesized(expr) {
        Expression::Variable(_) => true,
        Expression::PropertyLookup { base, .. } => is_reference(base),
        _ => false,
    }
}

/// Collects the references of an aggregating expression, outside its
/// aggregate calls, that are neither grouping keys nor local variables.
struct UngroupedReferences<'a> {
    is_aggregate: &'a dyn Fn(&FunctionInvocation) -> bool,
    /// Whether every variable is a grouping key (`WITH *`, `RETURN *`).
    star: bool,
    /// The printed grouping-key expressions.
    keys: &'a HashSet<String>,
    /// Variables bound inside the expression (comprehensions, `reduce`).
    locals: Vec<String>,
    /// The printed offending references, in order of appearance.
    found: Vec<String>,
}

impl UngroupedReferences<'_> {
    /// Visits `expr` with `variables` bound locally.
    fn visit_scoped(&mut self, variables: &[&Variable], expr: &Expression) {
        let depth = self.locals.len();
        self.locals
            .extend(variables.iter().map(|v| v.name.name.clone()));
        self.visit_expression(expr);
        self.locals.truncate(depth);
    }
}

impl<'ast> Visit<'ast> for UngroupedReferences<'_> {
    fn visit_expression(&mut self, node: &'ast Expression) {
        match node {
            Expression::CountStar { .. } => {}
            Expression::FunctionCall(fc) if (self.is_aggregate)(fc) => {}
            Expression::Variable(v) => {
                if !self.star
                    && !self.keys.contains(&node.to_cypher())
                    && !self.locals.contains(&v.name.name)
                {
                    self.found.push(v.name.name.clone());
                }
            }
            Expression::PropertyLookup { base, .. } => {
                if self.keys.contains(&node.to_cypher()) {
                    return;
                }
                // `n.a` is fine when `n` is a grouping key. Otherwise report
                // the whole access rather than its root variable.
                let before = self.found.len();
                self.visit_expression(base);
                if self.found.len() > before && is_reference(node) {
                    self.found.truncate(before);
                    self.found.push(node.to_cypher());
                }
            }
            Expression::ListComprehension(lc) => {
                self.visit_expression(&lc.collection);
                if let Some(filter) = &lc.filter {
                    self.visit_scoped(&[&lc.variable], filter);
                }
                if let Some(map) = &lc.map {
                    self.visit_scoped(&[&lc.variable], map);
                }
            }
            Expression::All(fe)
            | Expression::Any(fe)
            | Expression::None(fe)
            | Expression::Single(fe) => {
                self.visit_expression(&fe.collection);
                if let Some(predicate) = &fe.predicate {
                    self.visit_scoped(&[&fe.variable], predicate);
                }
            }
            Expression::Reduce(r) => {
                self.visit_expression(&r.init);
                self.visit_expression(&r.list);
                self.visit_scoped(&[&r.accumulator, &r.variable], &r.expression);
            }
            // Patterns and subqueries bind their own variables; their
            // references to the outer query are not checked.
            Expression::Pattern(_)
            | Expression::PatternComprehension(_)
            | Expression::Exists(_)
            | Expression::CountSubquery(_)
            | Expression::CollectSubquery(_) => {}
            _ => walk_expression(self, node),
        }
    }
}

/// The aggregate calls of `query` in positions that do not allow them, by
/// dot-joined function name and span.
///
/// Aggregates may appear in `WITH` and `RETURN` items, and in the `WHERE`
/// and `ORDER BY` of a projection that aggregates. They may not appear
/// anywhere else (`MATCH … WHERE count(n) > 1`, `CALL p(count(n))`), inside
/// the arguments of another aggregate (`count(count(*))`), or in the body of
/// a comprehension (`[x IN list | count(*)]`).
pub(crate) fn misplaced_aggregates(
    query: &Query,
    is_aggregate: &dyn Fn(&FunctionInvocation) -> bool,
) -> Vec<(String, Span)> {
    let mut placement = AggregatePlacement {
        is_aggregate,
        allowed: false,
        found: Vec::new(),
    };
    placement.visit_query(query);
    placement.found
}

/// Collects aggregate calls outside the positions that allow them.
struct AggregatePlacement<'a> {
    is_aggregate: &'a dyn Fn(&FunctionInvocation) -> bool,
    /// Whether the expression being visited may contain an aggregate.
    allowed: bool,
    /// Offending aggregate calls, by function name and span.
    found: Vec<(String, Span)>,
}

impl AggregatePlacement<'_> {
    /// Runs `visit` with aggregates `allowed` or not.
    fn with_allowed(&mut self, allowed: bool, visit: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.allowed, allowed);
        visit(self);
        self.allowed = outer;
    }

    fn visit_projection<'ast>(
        &mut self,
        items: &'ast [ProjectionItem],
        filters: &[Option<&'ast Expression>],
        order: Option<&'ast Order>,
        counts: &[Option<&'ast Expression>],
    ) {
        let aggregating = items
            .iter()
            .any(|item| item.expression.contains_aggregate(self.is_aggregate));
        self.with_allowed(true, |this| {
            for item in items {
                this.visit_expression(&item.expression);
            }
        });
        self.with_allowed(aggregating, |this| {
            for filter in filters.iter().flatten() {
                this.visit_expression(filter);
            }
            if let Some(order) = order {
                this.visit_order(order);
            }
        });
        self.with_allowed(false, |this| {
            for count in counts.iter().flatten() {
                this.visit_expression(count);
            }
        });
    }
}

impl<'ast> Visit<'ast> for AggregatePlacement<'_> {
    fn visit_with(&mut self, node: &'ast With) {
        self.visit_projection(
            &node.items,
            &[node.where_clause.as_ref()],
            node.order.as_ref(),
            &[node.skip.as_ref(), node.limit.as_ref()],
        );
    }

    fn visit_return(&mut self, node: &'ast Return) {
        self.visit_projection(
            &node.items,
            &[],
            node.order.as_ref(),
            &[node.skip.as_ref(), node.limit.as_ref()],
        );
    }

    fn visit_expression(&mut self, node: &'ast Expression) {
        let (name, span) = match node {
            Expression::CountStar { span } => ("count".to_string(), *span),
            Expression::FunctionCall(fc) if (self.is_aggregate)(fc) => {
                let name = fc
                    .name
                    .iter()
                    .map(|segment| segment.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                (name, fc.span)
            }
            _ => return walk_expression(self, node),
        };
        if !self.allowed {
            self.found.push((name, span));
        }
        self.with_allowed(false, |this| walk_expression(this, node));
    }

    fn visit_list_comprehension(&mut self, node: &'ast ListComprehension) {
        self.visit_expression(&node.collection);
        self.with_allowed(false, |this| {
            if let Some(filter) = &node.filter {
                this.visit_expression(filter);
            }
            if let Some(map) = &node.map {
                this.visit_expression(map);
            }
        });
    }

    fn visit_filter_expression(&mut self, node: &'ast FilterExpression) {
        self.visit_expression(&node.collection);
        if let Some(predicate) = &node.predicate {
            self.with_allowed(false, |this| this.visit_expression(predicate));
        }
    }

    fn visit_reduce_expression(&mut self, node: &'ast ReduceExpression) {
        self.visit_expression(&node.init);
        self.visit_expression(&node.list);
        self.with_allowed(false, |this| this.visit_expression(&node.expression));
    }

    fn visit_pattern_comprehension(&mut self, node: &'ast PatternComprehension) {
        self.with_allowed(false, |this| {
            crate::ast::visit::walk_pattern_comprehension(this, node)
        });
    }

    // Subqueries start without aggregates allowed; their own projections
    // allow them again.
    fn visit_exists_expression(&mut self, node: &'ast ExistsExpression) {
        self.with_allowed(false, |this| {
            crate::ast::visit::walk_exists_expression(this, node)
        });
    }

    fn visit_count_subquery_expression(&mut self, node: &'ast CountSubqueryExpression) {
        self.with_allowed(false, |this| {
            crate::ast::visit::walk_count_subquery_expression(this, node)
        });
    }

    fn visit_collect_subquery_expression(&mut self, node: &'ast CollectSubqueryExpression) {
        self.with_allowed(false, |this| {
            crate::ast::visit::walk_collect_subquery_expression(this, node)
        });
    }
}
