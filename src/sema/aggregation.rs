//! Aggregation rule validation.
//!
//! Rules enforced:
//! - WITH/RETURN may not mix aggregates and non-grouping expressions unless
//!   every non-aggregate is in the grouping key.
//! - DISTINCT only allowed in a projection body.
//! - Literals and parameters are always valid alongside aggregates.

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::pattern::PatternElement;
use crate::ast::query::*;
use crate::error::{Diagnostics, Span};
use crate::sema::error::SemaError;
use std::collections::HashSet;

/// A diagnostic violation detected during aggregation checking.
pub struct AggregationViolation;

/// Check aggregation rules for a query.
pub fn check_aggregation(query: &Query, diagnostics: &mut Diagnostics) {
    let mut checker = AggregationChecker { diagnostics };
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

struct AggregationChecker<'a> {
    diagnostics: &'a mut Diagnostics,
}

impl AggregationChecker<'_> {
    fn check_single_query(&mut self, sq: &SingleQuery) {
        match &sq.kind {
            SingleQueryKind::SinglePart(sp) => self.check_single_part(sp),
            SingleQueryKind::MultiPart(mp) => self.check_multi_part(mp),
        }
    }

    fn check_single_part(&mut self, sp: &SinglePartQuery) {
        // Collect grouping keys from reading clauses (variables bound by MATCH/UNWIND)
        let grouping_keys = collect_grouping_keys(&sp.reading_clauses);

        match &sp.body {
            SinglePartBody::Return(ret) => {
                self.check_projection(&ret.items, ret.distinct, &grouping_keys, ret.span);
            }
            SinglePartBody::Updating { return_clause, .. } => {
                if let Some(ret) = return_clause {
                    self.check_projection(&ret.items, ret.distinct, &grouping_keys, ret.span);
                }
            }
            SinglePartBody::Finish(_) => {}
        }
    }

    fn check_multi_part(&mut self, mp: &MultiPartQuery) {
        let mut grouping_keys: HashSet<String> = HashSet::new();

        for part in &mp.parts {
            // Each part's reading clauses contribute to grouping keys
            let part_keys = collect_grouping_keys(&part.reading_clauses);
            grouping_keys.extend(part_keys);

            // Derive effective column names from the WITH projection
            let mut with_keys: HashSet<String> = if part.with.star {
                grouping_keys.clone()
            } else {
                HashSet::new()
            };
            for item in &part.with.items {
                if let Some(name) = projection_column_name(item) {
                    with_keys.insert(name);
                }
            }

            // Check if WITH mixes aggregates and non-aggregates
            let has_agg = part
                .with
                .items
                .iter()
                .any(|item| has_aggregate(&item.expression));
            if has_agg {
                let non_grouping: Vec<String> = part
                    .with
                    .items
                    .iter()
                    .filter_map(|item| {
                        if !has_aggregate(&item.expression)
                            && !is_literal_or_param(&item.expression)
                        {
                            projection_column_name(item)
                        } else {
                            None
                        }
                    })
                    .filter(|name| !grouping_keys.contains(name))
                    .collect();
                if !non_grouping.is_empty() {
                    self.diagnostics.errors.push(
                        SemaError::AggregationMix {
                            non_grouping: non_grouping.clone(),
                            span: part.with.span,
                        }
                        .into_error(),
                    );
                }
            }

            grouping_keys = with_keys;
        }

        // Check final part
        let part_keys = collect_grouping_keys(&mp.final_part.reading_clauses);
        grouping_keys.extend(part_keys);

        match &mp.final_part.body {
            SinglePartBody::Return(ret) => {
                self.check_projection(&ret.items, ret.distinct, &grouping_keys, ret.span);
            }
            SinglePartBody::Updating { return_clause, .. } => {
                if let Some(ret) = return_clause {
                    self.check_projection(&ret.items, ret.distinct, &grouping_keys, ret.span);
                }
            }
            SinglePartBody::Finish(_) => {}
        }
    }

    fn check_projection(
        &mut self,
        items: &[ProjectionItem],
        distinct: bool,
        grouping_keys: &HashSet<String>,
        span: Span,
    ) {
        if distinct {
            // Check if any item is an aggregate - if so, this is fine (DISTINCT + aggregate is common)
            // If no aggregates and all items are in grouping keys or are aggregates, also fine
        }

        let has_agg = items.iter().any(|item| has_aggregate(&item.expression));
        if !has_agg {
            return; // No aggregation, nothing to check
        }

        // Has aggregates - check that non-aggregate, non-literal items are in grouping keys
        let non_grouping: Vec<String> = items
            .iter()
            .filter_map(|item| {
                if !has_aggregate(&item.expression) && !is_literal_or_param(&item.expression) {
                    let col = projection_column_name(item);
                    let var = extract_variable_name(&item.expression);
                    let is_grouped = col.as_ref().is_some_and(|n| grouping_keys.contains(n))
                        || var.as_ref().is_some_and(|n| grouping_keys.contains(n));
                    if is_grouped { None } else { col.or(var) }
                } else {
                    None
                }
            })
            .collect();

        if !non_grouping.is_empty() {
            self.diagnostics
                .errors
                .push(SemaError::AggregationMix { non_grouping, span }.into_error());
        }
    }
}

fn collect_grouping_keys(clauses: &[ReadingClause]) -> HashSet<String> {
    let mut keys = HashSet::new();
    for clause in clauses {
        match clause {
            ReadingClause::Match(m) => {
                for part in &m.pattern.parts {
                    if let Some(var) = &part.variable {
                        keys.insert(var.name.name.clone());
                    }
                    collect_element_vars(&part.anonymous.element, &mut keys);
                }
            }
            ReadingClause::Unwind(u) => {
                keys.insert(u.variable.name.name.clone());
            }
            ReadingClause::InQueryCall(_) | ReadingClause::CallSubquery(_) => {}
            ReadingClause::LoadCsv(lc) => {
                keys.insert(lc.variable.name.name.clone());
            }
        }
    }
    keys
}

fn collect_element_vars(element: &PatternElement, keys: &mut HashSet<String>) {
    match element {
        PatternElement::Path { start, chains } => {
            if let Some(var) = &start.variable {
                keys.insert(var.name.name.clone());
            }
            for chain in chains {
                if let Some(var) = chain
                    .relationship
                    .detail
                    .as_ref()
                    .and_then(|d| d.variable.as_ref())
                {
                    keys.insert(var.name.name.clone());
                }
                if let Some(var) = &chain.node.variable {
                    keys.insert(var.name.name.clone());
                }
            }
        }
        PatternElement::Parenthesized(inner) => {
            collect_element_vars(inner, keys);
        }
        PatternElement::Quantified { element, .. } => {
            collect_element_vars(element, keys);
        }
    }
}

fn projection_column_name(item: &ProjectionItem) -> Option<String> {
    if let Some(alias) = &item.alias {
        return Some(alias.name.name.clone());
    }
    match &item.expression {
        Expression::Variable(v) => Some(v.name.name.clone()),
        Expression::PropertyLookup { property, .. } => Some(property.name.name.clone()),
        _ => None,
    }
}

fn has_aggregate(expr: &Expression) -> bool {
    match expr {
        Expression::FunctionCall(fc) => {
            // Check for aggregate functions (COUNT, SUM, AVG, MIN, MAX, COLLECT, etc.)
            let name = fc
                .name
                .last()
                .map(|s| s.name.to_uppercase())
                .unwrap_or_default();
            matches!(
                name.as_str(),
                "COUNT"
                    | "SUM"
                    | "AVG"
                    | "MIN"
                    | "MAX"
                    | "COLLECT"
                    | "PERCENTILE_CONT"
                    | "PERCENTILE_DISC"
                    | "STDEV"
                    | "STDEVP"
                    | "VAR"
                    | "VARP"
            )
        }
        Expression::CountStar { .. } => true,
        Expression::BinaryOp {
            op: _, lhs, rhs, ..
        } => has_aggregate(lhs) || has_aggregate(rhs),
        Expression::UnaryOp { op: _, operand, .. } => has_aggregate(operand),
        Expression::Comparison { lhs, operators, .. } => {
            has_aggregate(lhs) || operators.iter().any(|(_, rhs)| has_aggregate(rhs))
        }
        Expression::Case(case) => {
            case.alternatives
                .iter()
                .any(|alt| has_aggregate(&alt.when) || has_aggregate(&alt.then))
                || case.scrutinee.as_ref().is_some_and(|s| has_aggregate(s))
                || case.default.as_ref().is_some_and(|d| has_aggregate(d))
        }
        Expression::Parenthesized(inner) => has_aggregate(inner),
        _ => false,
    }
}

fn is_literal_or_param(expr: &Expression) -> bool {
    matches!(expr, Expression::Literal(_) | Expression::Parameter(_))
}

fn extract_variable_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(v) => Some(v.name.name.clone()),
        Expression::PropertyLookup { base, .. } => extract_variable_name(base),
        _ => None,
    }
}
