//! Aggregation rule validation.
//!
//! Rules enforced:
//! - `WITH`/`RETURN` may not mix aggregates and non-grouping expressions
//!   unless every non-aggregate is in the grouping key.
//! - `DISTINCT` is only allowed inside a projection body.
//! - Literals and parameters are always valid alongside aggregates.
//!
//! The entry point is [`check_aggregation`].

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::pattern::PatternElement;
use crate::ast::query::*;
use crate::error::{Diagnostics, Span};
use crate::sema::error::SemaError;
use std::collections::HashSet;

/// A marker type for an aggregation-rule violation.
///
/// Currently unused as a concrete value; violations are reported directly
/// through the [`Diagnostics`] collector.
pub struct AggregationViolation;

/// Check aggregation rules for all statements in `query`.
///
/// Appends any violations found to `diagnostics`.
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

/// Internal checker that walks the query and emits aggregation diagnostics.
struct AggregationChecker<'a> {
    /// The diagnostics collector to which violations are appended.
    diagnostics: &'a mut Diagnostics,
}

impl AggregationChecker<'_> {
    /// Dispatch to the appropriate checker for a single or multi-part query.
    fn check_single_query(&mut self, sq: &SingleQuery) {
        match &sq.kind {
            SingleQueryKind::SinglePart(sp) => self.check_single_part(sp),
            SingleQueryKind::MultiPart(mp) => self.check_multi_part(mp),
        }
    }

    /// Check aggregation rules for a single-part query.
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

    /// Check aggregation rules for a multi-part (WITH-joined) query.
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

    /// Check that a projection (WITH/RETURN item list) does not mix aggregate
    /// and non-grouping-key expressions.
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

/// Collect all variables bound by the given reading clauses into a set.
///
/// These variable names form the valid grouping keys for a subsequent
/// aggregate projection.
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

/// Recursively collect all variable bindings introduced by a pattern element.
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

/// Return the output column name for a projection item.
///
/// Uses the explicit `AS alias` if present, otherwise infers the name from
/// a plain variable or the last property key in a lookup chain.
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

/// Return `true` if `expr` contains or is an aggregate function call.
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

/// Return `true` if `expr` is a literal value or a query parameter.
///
/// These expressions are always acceptable alongside aggregates because
/// they do not reference row-level variables.
fn is_literal_or_param(expr: &Expression) -> bool {
    matches!(expr, Expression::Literal(_) | Expression::Parameter(_))
}

/// Attempt to extract a simple variable name from an expression.
///
/// Returns `Some(name)` for plain variable references and the base of a
/// property-lookup chain; `None` for all other expression forms.
fn extract_variable_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(v) => Some(v.name.name.clone()),
        Expression::PropertyLookup { base, .. } => extract_variable_name(base),
        _ => None,
    }
}
