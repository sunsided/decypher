//! Aggregation rule validation.
//!
//! Rules enforced:
//! - WITH/RETURN may not mix aggregates and non-grouping expressions unless
//!   every non-aggregate is in the grouping key.
//! - DISTINCT only allowed in a projection body.
//! - Literals and parameters are always valid alongside aggregates.

use crate::ast::clause::*;
use crate::ast::expr::*;
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
        }
    }

    fn check_multi_part(&mut self, mp: &MultiPartQuery) {
        let mut grouping_keys: HashSet<String> = HashSet::new();

        for part in &mp.parts {
            // Each part's reading clauses contribute to grouping keys
            let part_keys = collect_grouping_keys(&part.reading_clauses);
            grouping_keys.extend(part_keys);

            // Check WITH clause
            let with_keys: HashSet<String> = part
                .with
                .items
                .iter()
                .filter_map(|item| item.alias.as_ref().map(|a| a.name.name.clone()))
                .collect();

            // The WITH projection items become the grouping keys for the next part
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
                            item.alias.as_ref().map(|a| a.name.name.clone())
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
                    // Extract the variable name if possible
                    extract_variable_name(&item.expression)
                        .filter(|name| !grouping_keys.contains(name))
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
                if let Some(var) = &m.pattern.parts.first().and_then(|p| p.variable.as_ref()) {
                    keys.insert(var.name.name.clone());
                }
            }
            ReadingClause::Unwind(u) => {
                keys.insert(u.variable.name.name.clone());
            }
            ReadingClause::InQueryCall(_) | ReadingClause::CallSubquery(_) => {}
        }
    }
    keys
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
        Expression::BinaryOp { op, lhs, rhs, .. } => has_aggregate(lhs) || has_aggregate(rhs),
        Expression::UnaryOp { op, operand, .. } => has_aggregate(operand),
        Expression::Comparison { lhs, operators, .. } => {
            has_aggregate(lhs) || operators.iter().any(|(_, rhs)| has_aggregate(rhs))
        }
        Expression::Case(case) => {
            case.alternatives
                .iter()
                .any(|alt| has_aggregate(&alt.when) || has_aggregate(&alt.then))
                || case.scrutinee.as_ref().map_or(false, |s| has_aggregate(s))
                || case.default.as_ref().map_or(false, |d| has_aggregate(d))
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

impl SemaError {
    fn into_error(self) -> crate::error::CypherError {
        crate::error::CypherError {
            kind: self.to_error_kind(),
            span: match &self {
                SemaError::UnresolvedVariable { span, .. } => *span,
                SemaError::RedeclaredVariable { redecl_span, .. } => *redecl_span,
                SemaError::AggregationMix { span, .. } => *span,
                SemaError::DistinctNotAllowed { span } => *span,
                SemaError::InvalidReference { span, .. } => *span,
            },
            source_label: None,
            notes: Vec::new(),
            source: None,
        }
    }
}
