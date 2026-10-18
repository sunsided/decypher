//! Structural rules that do not depend on scope or types.
//!
//! - All branches of a `UNION` return the same columns, and a query does not
//!   mix `UNION` with `UNION ALL`.
//! - `EXISTS`, `COUNT` and `COLLECT` subqueries do not update the graph.
//! - A relationship created by `CREATE` or `MERGE` has exactly one type and
//!   a single hop; one created by `CREATE` is directed.
//! - A `MATCH` pattern uses each relationship variable once.
//! - `DELETE` targets are nodes, relationships or paths, not label checks.
//! - `WITH` items other than variables are aliased.
//! - `SKIP` and `LIMIT` take a constant, non-negative integer.
//! - `MATCH` and `MERGE` patterns use literal property maps, not parameters
//!   (`MATCH (n $param)`).
//! - Aggregates do not call `rand()`.
//! - Pattern expressions are predicates: they appear in `WHERE`, under
//!   `NOT` / `AND` / `OR` / `XOR`, in `exists(…)` and as `CASE WHEN`
//!   conditions, not as values (`RETURN size((n)-->())`).
//! - Values have the types their operators, clauses and functions accept,
//!   and calls match their [`Catalog`] signatures; a variable is used as one
//!   kind of entity only (see [`super::types`]).
//!
//! The entry point is [`structural_errors`], shared by
//! [`crate::sema::analyze`] and HIR lowering.

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::pattern::*;
use crate::ast::query::*;
use crate::ast::visit::{
    Visit, walk_collect_subquery_expression, walk_count_subquery_expression, walk_create,
    walk_delete, walk_exists_expression, walk_expression, walk_function_invocation_expr,
    walk_merge, walk_node_pattern, walk_regular_query, walk_relationship_detail, walk_remove,
    walk_return, walk_set,
};
use std::borrow::Cow;

use super::Catalog;
use crate::error::{ErrorKind, Span, ValueTypes, VariableKind};

/// A violation of a structural rule.
#[derive(Debug, Clone)]
pub(crate) enum StructuralError {
    /// `UNION` branches return different columns.
    UnionColumnMismatch { span: Span },
    /// Clauses that cannot be combined (`UNION` with `UNION ALL`, updates in
    /// an `EXISTS` subquery).
    InvalidClauseComposition { reason: &'static str, span: Span },
    /// A relationship pattern that its clause cannot use.
    InvalidRelationshipPattern { reason: &'static str, span: Span },
    /// A `DELETE` target that is a label check (`DELETE n:Person`).
    InvalidDeleteTarget { span: Span },
    /// A `WITH` item that is neither a variable nor aliased.
    MissingAlias { span: Span },
    /// A `SKIP` or `LIMIT` value that is not a constant, non-negative
    /// integer.
    InvalidSkipLimit { reason: &'static str, span: Span },
    /// A parameter used as the property map of a `MATCH` or `MERGE` pattern.
    InvalidParameterUse { span: Span },
    /// A non-deterministic function called inside an aggregate
    /// (`count(rand())`).
    NonDeterministicAggregate { span: Span },
    /// A pattern expression used as a value (`RETURN (n)-->()`).
    PatternExpressionOutsidePredicate { span: Span },
    /// A value of a type its operator, clause or function does not accept.
    InvalidArgumentType {
        context: Cow<'static, str>,
        expected: ValueTypes,
        found: ValueTypes,
        span: Span,
    },
    /// A call with too few or too many arguments.
    InvalidNumberOfArguments {
        name: String,
        min: u16,
        max: Option<u16>,
        found: u16,
        span: Span,
    },
    /// A function a strict catalog does not know.
    UnknownFunction { name: String, span: Span },
    /// A procedure a strict catalog does not know.
    ProcedureNotFound { name: String, span: Span },
    /// A variable used as two kinds of entity (`MATCH (r) MATCH ()-[r]-()`).
    VariableTypeConflict {
        name: String,
        bound_as: VariableKind,
        used_as: VariableKind,
        span: Span,
    },
}

impl StructuralError {
    /// The public error kind of this violation.
    pub(crate) fn kind(&self) -> ErrorKind {
        match self {
            Self::UnionColumnMismatch { .. } => ErrorKind::UnionColumnMismatch,
            Self::InvalidClauseComposition { reason, .. } => {
                ErrorKind::InvalidClauseComposition { reason }
            }
            Self::InvalidRelationshipPattern { reason, .. } => {
                ErrorKind::InvalidRelationshipPattern { reason }
            }
            Self::InvalidDeleteTarget { .. } => ErrorKind::InvalidDeleteTarget,
            Self::MissingAlias { .. } => ErrorKind::MissingAlias,
            Self::InvalidSkipLimit { reason, .. } => ErrorKind::InvalidSkipLimit { reason },
            Self::InvalidParameterUse { .. } => ErrorKind::InvalidParameterUse,
            Self::NonDeterministicAggregate { .. } => ErrorKind::NonDeterministicAggregate,
            Self::PatternExpressionOutsidePredicate { .. } => {
                ErrorKind::PatternExpressionOutsidePredicate
            }
            Self::InvalidArgumentType {
                context,
                expected,
                found,
                ..
            } => ErrorKind::InvalidArgumentType {
                context: context.clone(),
                expected: *expected,
                found: *found,
            },
            Self::InvalidNumberOfArguments {
                name,
                min,
                max,
                found,
                ..
            } => ErrorKind::InvalidNumberOfArguments {
                name: name.clone(),
                min: *min,
                max: *max,
                found: *found,
            },
            Self::UnknownFunction { name, .. } => ErrorKind::UnknownFunction { name: name.clone() },
            Self::ProcedureNotFound { name, .. } => {
                ErrorKind::ProcedureNotFound { name: name.clone() }
            }
            Self::VariableTypeConflict {
                name,
                bound_as,
                used_as,
                ..
            } => ErrorKind::VariableTypeConflict {
                name: name.clone(),
                bound_as: *bound_as,
                used_as: *used_as,
            },
        }
    }

    /// Where the violation is.
    pub(crate) fn span(&self) -> Span {
        match self {
            Self::UnionColumnMismatch { span }
            | Self::InvalidClauseComposition { span, .. }
            | Self::InvalidRelationshipPattern { span, .. }
            | Self::InvalidDeleteTarget { span }
            | Self::MissingAlias { span }
            | Self::InvalidSkipLimit { span, .. }
            | Self::InvalidParameterUse { span }
            | Self::NonDeterministicAggregate { span }
            | Self::PatternExpressionOutsidePredicate { span }
            | Self::InvalidArgumentType { span, .. }
            | Self::InvalidNumberOfArguments { span, .. }
            | Self::UnknownFunction { span, .. }
            | Self::ProcedureNotFound { span, .. }
            | Self::VariableTypeConflict { span, .. } => *span,
        }
    }
}

/// The structural and type violations in `query`, checking calls against
/// `catalog`.
pub(crate) fn structural_errors(query: &Query, catalog: &Catalog) -> Vec<StructuralError> {
    let mut checker = StructureChecker {
        catalog,
        read_only: false,
        creating: false,
        in_aggregate: false,
        predicate: false,
        errors: Vec::new(),
    };
    checker.visit_query(query);
    let mut errors = checker.errors;
    errors.extend(super::types::type_errors(query, catalog));
    errors
}

struct StructureChecker<'a> {
    catalog: &'a Catalog,
    /// Whether the clauses being visited belong to an `EXISTS`, `COUNT` or
    /// `COLLECT` subquery, which may not update the graph.
    read_only: bool,
    /// Whether the patterns being visited belong to a `CREATE`, which may
    /// take a parameter as property map.
    creating: bool,
    /// Whether the expressions being visited are arguments of an aggregate.
    in_aggregate: bool,
    /// Whether the expression being visited is a predicate, where a pattern
    /// expression may appear.
    predicate: bool,
    errors: Vec<StructuralError>,
}

impl StructureChecker<'_> {
    fn report_update(&mut self, span: Span) {
        if self.read_only {
            self.errors.push(StructuralError::InvalidClauseComposition {
                reason: "EXISTS, COUNT and COLLECT subqueries cannot update the graph",
                span,
            });
        }
    }

    fn check_skip_limit(&mut self, skip: Option<&Expression>, limit: Option<&Expression>) {
        for (clause, expr) in [("SKIP", skip), ("LIMIT", limit)] {
            let Some(expr) = expr else { continue };
            let reason = if references_variable(expr) {
                Some(match clause {
                    "SKIP" => "SKIP cannot refer to variables",
                    _ => "LIMIT cannot refer to variables",
                })
            } else {
                match literal_number(expr) {
                    Some(NumberLiteral::Integer(n)) if n < 0 => Some(match clause {
                        "SKIP" => "SKIP cannot be negative",
                        _ => "LIMIT cannot be negative",
                    }),
                    Some(NumberLiteral::Float(_)) => Some(match clause {
                        "SKIP" => "SKIP must be an integer",
                        _ => "LIMIT must be an integer",
                    }),
                    _ => None,
                }
            };
            if let Some(reason) = reason {
                self.errors.push(StructuralError::InvalidSkipLimit {
                    reason,
                    span: expr.span(),
                });
            }
        }
    }

    fn check_properties(&mut self, properties: Option<&Properties>) {
        if let Some(Properties::Parameter(param)) = properties
            && !self.creating
        {
            self.errors
                .push(StructuralError::InvalidParameterUse { span: param.span });
        }
    }

    fn visit_predicate(&mut self, expr: &Expression) {
        let outer = std::mem::replace(&mut self.predicate, true);
        self.visit_expression(expr);
        self.predicate = outer;
    }

    fn visit_read_only(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.read_only, true);
        visit(self);
        self.read_only = outer;
    }

    /// Checks the relationships of a `CREATE` (`directed`) or `MERGE`
    /// pattern element.
    fn check_created(&mut self, element: &PatternElement, directed: bool) {
        for rel in relationships(element) {
            let reason = match &rel.detail {
                _ if directed
                    && matches!(
                        rel.direction,
                        RelationshipDirection::Both | RelationshipDirection::Undirected
                    ) =>
                {
                    Some("CREATE needs a direction for every relationship")
                }
                Some(detail) if detail.range.is_some() || rel.quantifier.is_some() => {
                    Some("CREATE and MERGE cannot create variable-length relationships")
                }
                Some(RelationshipDetail {
                    types: Some(types), ..
                }) if is_single_type(types) => None,
                _ => Some("CREATE and MERGE need exactly one type for every relationship"),
            };
            if let Some(reason) = reason {
                self.errors
                    .push(StructuralError::InvalidRelationshipPattern {
                        reason,
                        span: rel.span,
                    });
            }
        }
    }
}

impl<'ast> Visit<'ast> for StructureChecker<'_> {
    fn visit_regular_query(&mut self, node: &'ast RegularQuery) {
        if node.unions.iter().any(|u| u.all) && node.unions.iter().any(|u| !u.all) {
            self.errors.push(StructuralError::InvalidClauseComposition {
                reason: "UNION and UNION ALL cannot be combined",
                span: node.unions[0].span,
            });
        }
        if let Some(first) = union_columns(&node.single_query) {
            for union in &node.unions {
                if union_columns(&union.single_query).is_some_and(|columns| columns != first) {
                    self.errors
                        .push(StructuralError::UnionColumnMismatch { span: union.span });
                }
            }
        }
        walk_regular_query(self, node);
    }

    fn visit_match(&mut self, node: &'ast Match) {
        let mut seen: Vec<&str> = Vec::new();
        for part in &node.pattern.parts {
            for rel in relationships(&part.anonymous.element) {
                let Some(var) = rel.detail.as_ref().and_then(|d| d.variable.as_ref()) else {
                    continue;
                };
                if seen.contains(&var.name.name.as_str()) {
                    self.errors
                        .push(StructuralError::InvalidRelationshipPattern {
                            reason: "a MATCH pattern cannot use the same relationship twice",
                            span: var.name.span,
                        });
                } else {
                    seen.push(&var.name.name);
                }
            }
        }
        self.visit_pattern(&node.pattern);
        if let Some(predicate) = &node.where_clause {
            self.visit_predicate(predicate);
        }
    }

    fn visit_create(&mut self, node: &'ast Create) {
        self.report_update(node.span);
        for part in &node.pattern.parts {
            self.check_created(&part.anonymous.element, true);
        }
        let outer = std::mem::replace(&mut self.creating, true);
        walk_create(self, node);
        self.creating = outer;
    }

    fn visit_merge(&mut self, node: &'ast Merge) {
        self.report_update(node.span);
        self.check_created(&node.pattern.anonymous.element, false);
        walk_merge(self, node);
    }

    fn visit_delete(&mut self, node: &'ast Delete) {
        self.report_update(node.span);
        for target in &node.targets {
            if matches!(target, Expression::NodeLabels { .. }) {
                self.errors.push(StructuralError::InvalidDeleteTarget {
                    span: target.span(),
                });
            }
        }
        walk_delete(self, node);
    }

    fn visit_set(&mut self, node: &'ast Set) {
        self.report_update(node.span);
        walk_set(self, node);
    }

    fn visit_remove(&mut self, node: &'ast Remove) {
        self.report_update(node.span);
        walk_remove(self, node);
    }

    fn visit_node_pattern(&mut self, node: &'ast NodePattern) {
        self.check_properties(node.properties.as_ref());
        walk_node_pattern(self, node);
    }

    fn visit_relationship_detail(&mut self, node: &'ast RelationshipDetail) {
        self.check_properties(node.properties.as_ref());
        walk_relationship_detail(self, node);
    }

    fn visit_function_invocation_expr(&mut self, node: &'ast FunctionInvocation) {
        if self.in_aggregate
            && let [name] = node.name.as_slice()
            && name.name.eq_ignore_ascii_case("rand")
        {
            self.errors
                .push(StructuralError::NonDeterministicAggregate { span: node.span });
        }
        let outer = self.in_aggregate;
        self.in_aggregate |= self.catalog.is_aggregate_call(node);
        walk_function_invocation_expr(self, node);
        self.in_aggregate = outer;
    }

    fn visit_return(&mut self, node: &'ast Return) {
        self.check_skip_limit(node.skip.as_ref(), node.limit.as_ref());
        walk_return(self, node);
    }

    fn visit_with(&mut self, node: &'ast With) {
        self.check_skip_limit(node.skip.as_ref(), node.limit.as_ref());
        for item in &node.items {
            if item.alias.is_none() && !matches!(item.expression, Expression::Variable(_)) {
                self.errors.push(StructuralError::MissingAlias {
                    span: item.expression.span(),
                });
            }
        }
        for item in &node.items {
            self.visit_projection_item(item);
        }
        if let Some(order) = &node.order {
            self.visit_order(order);
        }
        for expr in [&node.skip, &node.limit].into_iter().flatten() {
            self.visit_expression(expr);
        }
        if let Some(predicate) = &node.where_clause {
            self.visit_predicate(predicate);
        }
    }

    fn visit_expression(&mut self, node: &'ast Expression) {
        let outer = self.predicate;
        if let Expression::Pattern(pattern) = node
            && !outer
        {
            self.errors
                .push(StructuralError::PatternExpressionOutsidePredicate { span: pattern.span });
        }
        self.predicate = match node {
            Expression::BinaryOp {
                op: BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor,
                ..
            }
            | Expression::UnaryOp {
                op: UnaryOperator::Not,
                ..
            }
            | Expression::Parenthesized(_) => outer,
            Expression::FunctionCall(call) => {
                matches!(call.name.as_slice(), [name] if name.name.eq_ignore_ascii_case("exists"))
            }
            _ => false,
        };
        walk_expression(self, node);
        self.predicate = outer;
    }

    fn visit_exists_inner(&mut self, node: &'ast ExistsInner) {
        match node {
            ExistsInner::Pattern(pattern, predicate) => {
                self.visit_pattern(pattern);
                if let Some(predicate) = predicate {
                    self.visit_predicate(predicate);
                }
            }
            ExistsInner::RegularQuery(query) => self.visit_regular_query(query),
        }
    }

    fn visit_case_expression(&mut self, node: &'ast CaseExpression) {
        // `WHEN` is a predicate only in the searched form (`CASE WHEN p`);
        // in `CASE x WHEN v` it is a value compared with `x`.
        let searched = node.scrutinee.is_none();
        if let Some(scrutinee) = &node.scrutinee {
            self.visit_expression(scrutinee);
        }
        for alternative in &node.alternatives {
            if searched {
                self.visit_predicate(&alternative.when);
            } else {
                self.visit_expression(&alternative.when);
            }
            self.visit_expression(&alternative.then);
        }
        if let Some(default) = &node.default {
            self.visit_expression(default);
        }
    }

    fn visit_list_comprehension(&mut self, node: &'ast ListComprehension) {
        self.visit_expression(&node.collection);
        if let Some(predicate) = &node.filter {
            self.visit_predicate(predicate);
        }
        if let Some(map) = &node.map {
            self.visit_expression(map);
        }
    }

    fn visit_pattern_comprehension(&mut self, node: &'ast PatternComprehension) {
        self.visit_relationships_pattern(&node.pattern);
        if let Some(predicate) = &node.where_clause {
            self.visit_predicate(predicate);
        }
        self.visit_expression(&node.map);
    }

    fn visit_filter_expression(&mut self, node: &'ast FilterExpression) {
        self.visit_expression(&node.collection);
        if let Some(predicate) = &node.predicate {
            self.visit_predicate(predicate);
        }
    }

    fn visit_exists_expression(&mut self, node: &'ast ExistsExpression) {
        self.visit_read_only(|this| walk_exists_expression(this, node));
    }

    fn visit_count_subquery_expression(&mut self, node: &'ast CountSubqueryExpression) {
        self.visit_read_only(|this| walk_count_subquery_expression(this, node));
    }

    fn visit_collect_subquery_expression(&mut self, node: &'ast CollectSubqueryExpression) {
        self.visit_read_only(|this| walk_collect_subquery_expression(this, node));
    }
}

/// The sorted column names of a `UNION` branch, or `None` when they are not
/// known statically (`RETURN *`) or it returns nothing.
fn union_columns(query: &SingleQuery) -> Option<Vec<String>> {
    let body = match &query.kind {
        SingleQueryKind::SinglePart(sp) => &sp.body,
        SingleQueryKind::MultiPart(mp) => &mp.final_part.body,
    };
    let ret = match body {
        SinglePartBody::Return(ret) => ret,
        SinglePartBody::Updating {
            return_clause: Some(ret),
            ..
        } => ret,
        _ => return None,
    };
    if ret.star {
        return None;
    }
    let mut columns: Vec<String> = ret.items.iter().map(|item| item.column().0).collect();
    columns.sort();
    Some(columns)
}

/// Whether `expr` refers to a variable.
fn references_variable(expr: &Expression) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_variable(&mut self, _: &'ast crate::ast::names::Variable) {
            self.0 = true;
        }
    }
    let mut finder = Finder(false);
    finder.visit_expression(expr);
    finder.0
}

/// The value of a numeric literal, negated or not.
fn literal_number(expr: &Expression) -> Option<NumberLiteral> {
    match expr {
        Expression::Literal(Literal::Number(n)) => Some(n.clone()),
        Expression::Parenthesized(inner) => literal_number(inner),
        Expression::UnaryOp {
            op: UnaryOperator::Negate,
            operand,
            ..
        } => match literal_number(operand)? {
            NumberLiteral::Integer(n) => Some(NumberLiteral::Integer(n.checked_neg()?)),
            NumberLiteral::Float(f) => Some(NumberLiteral::Float(-f)),
        },
        _ => None,
    }
}

/// Whether a relationship type expression names exactly one static type.
fn is_single_type(types: &LabelExpression) -> bool {
    match types {
        LabelExpression::Static(_) => true,
        LabelExpression::Group { inner, .. } => is_single_type(inner),
        _ => false,
    }
}

/// The relationship patterns of a pattern element, in order.
fn relationships(element: &PatternElement) -> Vec<&RelationshipPattern> {
    match element {
        PatternElement::Path { chains, .. } => chains.iter().map(|c| &c.relationship).collect(),
        PatternElement::Parenthesized(inner) => relationships(inner),
        PatternElement::Quantified { element, .. } => relationships(element),
    }
}
