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
//!
//! The entry point is [`structural_errors`], shared by
//! [`crate::sema::analyze`] and HIR lowering.

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::pattern::*;
use crate::ast::query::*;
use crate::ast::visit::{
    Visit, walk_collect_subquery_expression, walk_count_subquery_expression, walk_create,
    walk_delete, walk_exists_expression, walk_match, walk_merge, walk_regular_query, walk_remove,
    walk_set, walk_with,
};
use crate::error::{ErrorKind, Span};

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
        }
    }

    /// Where the violation is.
    pub(crate) fn span(&self) -> Span {
        match self {
            Self::UnionColumnMismatch { span }
            | Self::InvalidClauseComposition { span, .. }
            | Self::InvalidRelationshipPattern { span, .. }
            | Self::InvalidDeleteTarget { span }
            | Self::MissingAlias { span } => *span,
        }
    }
}

/// The structural violations in `query`, in source order.
pub(crate) fn structural_errors(query: &Query) -> Vec<StructuralError> {
    let mut checker = StructureChecker {
        read_only: false,
        errors: Vec::new(),
    };
    checker.visit_query(query);
    checker.errors
}

struct StructureChecker {
    /// Whether the clauses being visited belong to an `EXISTS`, `COUNT` or
    /// `COLLECT` subquery, which may not update the graph.
    read_only: bool,
    errors: Vec<StructuralError>,
}

impl StructureChecker {
    fn report_update(&mut self, span: Span) {
        if self.read_only {
            self.errors.push(StructuralError::InvalidClauseComposition {
                reason: "EXISTS, COUNT and COLLECT subqueries cannot update the graph",
                span,
            });
        }
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

impl<'ast> Visit<'ast> for StructureChecker {
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
        walk_match(self, node);
    }

    fn visit_create(&mut self, node: &'ast Create) {
        self.report_update(node.span);
        for part in &node.pattern.parts {
            self.check_created(&part.anonymous.element, true);
        }
        walk_create(self, node);
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

    fn visit_with(&mut self, node: &'ast With) {
        for item in &node.items {
            if item.alias.is_none() && !matches!(item.expression, Expression::Variable(_)) {
                self.errors.push(StructuralError::MissingAlias {
                    span: item.expression.span(),
                });
            }
        }
        walk_with(self, node);
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
