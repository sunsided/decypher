//! Variables used as more than one kind of entity.
//!
//! A pattern position fixes the kind of its variable: `(n)` is a node,
//! `-[r]-` a relationship, `-[r*]-` a list of relationships and `p = …` a
//! path. A variable projected from a literal (`WITH 1 AS n`) is a value. Any
//! other use of the same variable as a different kind, such as
//! `MATCH (r)-->() MATCH ()-[r]-()`, is a conflict.
//!
//! The check follows Cypher's scoping without resolving names fully: `WITH`
//! and `RETURN` replace the visible variables with their columns, `UNION`
//! branches and `CALL { … }` bodies start empty, and subquery expressions see
//! the outer variables without changing them. A variable whose kind is not
//! known from a pattern or literal never conflicts.

use std::collections::HashMap;

use super::structure::StructuralError;
use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::names::Variable;
use crate::ast::pattern::*;
use crate::ast::query::*;
use crate::ast::visit::{
    Visit, walk_call_subquery, walk_create, walk_exists_inner, walk_expression,
    walk_filter_expression, walk_foreach, walk_list_comprehension, walk_match, walk_merge,
    walk_pattern_comprehension, walk_reduce_expression, walk_return, walk_with,
};
use crate::error::VariableKind;

/// Whether a variable bound as `bound` can be used as `used`.
fn fits(bound: VariableKind, used: VariableKind) -> bool {
    bound == used || (bound == VariableKind::List && used == VariableKind::Relationships)
}

/// The kinds of the visible variables; `None` when not known, which never
/// conflicts.
type Kinds = HashMap<String, Option<VariableKind>>;

/// The variables of `query` used as more than one kind of entity.
pub(super) fn variable_type_conflicts(query: &Query) -> Vec<StructuralError> {
    let mut checker = KindChecker::default();
    checker.visit_query(query);
    checker.errors
}

#[derive(Default)]
struct KindChecker {
    /// The kinds of the visible variables.
    kinds: Kinds,
    /// The variables a nested query starts with: none for a `CALL { … }`
    /// body, the outer ones for `EXISTS`, `COUNT` and `COLLECT`.
    next_start: Kinds,
    errors: Vec<StructuralError>,
}

impl KindChecker {
    /// Records that `var` is used as `kind`, reporting a conflict with an
    /// earlier use.
    fn bind(&mut self, var: &Variable, kind: VariableKind) {
        let name = &var.name.name;
        match self.kinds.get(name).copied().flatten() {
            Some(bound) if !fits(bound, kind) => {
                self.errors.push(StructuralError::VariableTypeConflict {
                    name: name.clone(),
                    bound_as: bound,
                    used_as: kind,
                    span: var.name.span,
                });
            }
            // A known kind stays: a list reused by `-[rs*]-` is still a list.
            Some(_) => {}
            None => {
                self.kinds.insert(name.clone(), Some(kind));
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern) {
        for part in &pattern.parts {
            self.bind_part(part);
        }
    }

    fn bind_part(&mut self, part: &PatternPart) {
        self.bind_element(&part.anonymous.element);
        // A path variable must be new; reusing one is a redeclaration.
        if let Some(var) = &part.variable {
            self.kinds
                .entry(var.name.name.clone())
                .or_insert(Some(VariableKind::Path));
        }
    }

    fn bind_element(&mut self, element: &PatternElement) {
        match element {
            PatternElement::Path { start, chains } => {
                self.bind_node(start);
                for chain in chains {
                    self.bind_relationship(&chain.relationship);
                    self.bind_node(&chain.node);
                }
            }
            PatternElement::Parenthesized(inner) => self.bind_element(inner),
            // Variables of a quantified path pattern are lists; not tracked.
            PatternElement::Quantified { .. } => {}
        }
    }

    fn bind_relationships_pattern(&mut self, pattern: &RelationshipsPattern) {
        self.bind_node(&pattern.start);
        for chain in &pattern.chains {
            self.bind_relationship(&chain.relationship);
            self.bind_node(&chain.node);
        }
    }

    fn bind_node(&mut self, node: &NodePattern) {
        if let Some(var) = &node.variable {
            self.bind(var, VariableKind::Node);
        }
    }

    fn bind_relationship(&mut self, rel: &RelationshipPattern) {
        let Some(detail) = &rel.detail else { return };
        let Some(var) = &detail.variable else { return };
        let kind = if detail.range.is_some() || rel.quantifier.is_some() {
            VariableKind::Relationships
        } else {
            VariableKind::Relationship
        };
        self.bind(var, kind);
    }

    /// Runs `visit` with the current variables, discarding what it binds.
    fn nested(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.kinds.clone();
        visit(self);
        self.kinds = outer;
    }

    /// Replaces the visible variables with the columns of a projection.
    fn project(&mut self, star: bool, items: &[ProjectionItem]) {
        let mut projected = if star {
            self.kinds.clone()
        } else {
            Kinds::new()
        };
        for item in items {
            let name = match (&item.alias, &item.expression) {
                (Some(alias), _) => alias.name.name.clone(),
                (None, Expression::Variable(var)) => var.name.name.clone(),
                _ => continue,
            };
            projected.insert(name, self.kind_of(&item.expression));
        }
        self.kinds = projected;
    }

    fn kind_of(&self, expr: &Expression) -> Option<VariableKind> {
        match expr {
            Expression::Variable(var) => self.kinds.get(&var.name.name).copied().flatten(),
            Expression::Literal(Literal::Null) => None,
            Expression::Literal(Literal::List(_)) => Some(VariableKind::List),
            Expression::Literal(_) => Some(VariableKind::Value),
            Expression::Parenthesized(inner) => self.kind_of(inner),
            _ => None,
        }
    }

    /// Hides `var` behind a local variable of unknown kind.
    fn shadow(&mut self, var: &Variable) {
        self.kinds.insert(var.name.name.clone(), None);
    }
}

impl<'ast> Visit<'ast> for KindChecker {
    fn visit_query(&mut self, node: &'ast Query) {
        for stmt in &node.statements {
            self.kinds.clear();
            self.next_start.clear();
            match stmt {
                QueryBody::SingleQuery(sq) => self.visit_single_query(sq),
                QueryBody::Regular(rq) => self.visit_regular_query(rq),
                _ => {}
            }
        }
    }

    fn visit_regular_query(&mut self, node: &'ast RegularQuery) {
        let start = std::mem::take(&mut self.next_start);
        let outer = std::mem::replace(&mut self.kinds, start.clone());
        self.visit_single_query(&node.single_query);
        // The columns of the first branch, for `CALL { … }`.
        let columns = std::mem::take(&mut self.kinds);
        for union in &node.unions {
            self.kinds = start.clone();
            self.visit_single_query(&union.single_query);
        }
        self.kinds = outer;
        self.next_start = columns;
    }

    fn visit_match(&mut self, node: &'ast Match) {
        self.bind_pattern(&node.pattern);
        walk_match(self, node);
    }

    fn visit_create(&mut self, node: &'ast Create) {
        self.bind_pattern(&node.pattern);
        walk_create(self, node);
    }

    fn visit_merge(&mut self, node: &'ast Merge) {
        self.bind_part(&node.pattern);
        walk_merge(self, node);
    }

    fn visit_with(&mut self, node: &'ast With) {
        walk_with(self, node);
        self.project(node.star, &node.items);
    }

    fn visit_return(&mut self, node: &'ast Return) {
        walk_return(self, node);
        self.project(node.star, &node.items);
    }

    fn visit_call_subquery(&mut self, node: &'ast CallSubquery) {
        self.next_start = Kinds::new();
        walk_call_subquery(self, node);
        let columns = std::mem::take(&mut self.next_start);
        if returns(&node.query) {
            self.kinds.extend(columns);
        }
    }

    fn visit_exists_inner(&mut self, node: &'ast ExistsInner) {
        self.nested(|this| {
            match node {
                ExistsInner::Pattern(pattern, _) => this.bind_pattern(pattern),
                ExistsInner::RegularQuery(_) => this.next_start = this.kinds.clone(),
            }
            walk_exists_inner(this, node);
        });
    }

    fn visit_collect_subquery_expression(&mut self, node: &'ast CollectSubqueryExpression) {
        self.nested(|this| {
            this.next_start = this.kinds.clone();
            this.visit_regular_query(&node.query);
        });
    }

    fn visit_expression(&mut self, node: &'ast Expression) {
        if let Expression::Pattern(pattern) = node {
            self.nested(|this| this.bind_relationships_pattern(pattern));
        }
        walk_expression(self, node);
    }

    fn visit_pattern_comprehension(&mut self, node: &'ast PatternComprehension) {
        self.nested(|this| {
            this.bind_relationships_pattern(&node.pattern);
            if let Some(var) = &node.variable {
                this.kinds
                    .entry(var.name.name.clone())
                    .or_insert(Some(VariableKind::Path));
            }
            walk_pattern_comprehension(this, node);
        });
    }

    fn visit_list_comprehension(&mut self, node: &'ast ListComprehension) {
        self.nested(|this| {
            this.shadow(&node.variable);
            walk_list_comprehension(this, node);
        });
    }

    fn visit_filter_expression(&mut self, node: &'ast FilterExpression) {
        self.nested(|this| {
            this.shadow(&node.variable);
            walk_filter_expression(this, node);
        });
    }

    fn visit_reduce_expression(&mut self, node: &'ast ReduceExpression) {
        self.nested(|this| {
            this.shadow(&node.accumulator);
            this.shadow(&node.variable);
            walk_reduce_expression(this, node);
        });
    }

    fn visit_foreach(&mut self, node: &'ast Foreach) {
        self.nested(|this| {
            this.shadow(&node.variable);
            walk_foreach(this, node);
        });
    }
}

/// Whether a query ends with `RETURN`, so that its columns are visible after
/// a `CALL { … }`.
fn returns(query: &RegularQuery) -> bool {
    let body = match &query.single_query.kind {
        SingleQueryKind::SinglePart(sp) => &sp.body,
        SingleQueryKind::MultiPart(mp) => &mp.final_part.body,
    };
    matches!(
        body,
        SinglePartBody::Return(_)
            | SinglePartBody::Updating {
                return_clause: Some(_),
                ..
            }
    )
}
