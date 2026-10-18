//! Static types: what operators, clauses and calls are given.
//!
//! Cypher is dynamically typed, so most types are only known at run time.
//! This pass tracks the types that are known statically: those of literals,
//! of pattern variables (`(n)` is a node, `-[r]-` a relationship, `-[r*]-` a
//! list of relationships, `p = …` a path) and of function results. It
//! reports:
//!
//! - a variable used as two kinds of entity (`MATCH (r)-->() MATCH ()-[r]-()`,
//!   `WITH 1 AS n MATCH (n)`);
//! - an operand, `WHERE` predicate or `DELETE` target of a type that cannot
//!   be used there (`NOT 'a'`, `1 IN 2`, `WHERE (n)`, `DELETE 1 + 1`);
//! - calls that do not match their [`Catalog`] signature (`labels(p)` on a
//!   path, `left('a')`), and, with a strict catalog, unknown functions and
//!   procedures.
//!
//! A value whose type is not known statically never conflicts.
//!
//! The pass follows Cypher's scoping without resolving names fully: `WITH`
//! and `RETURN` replace the visible variables with their columns, `UNION`
//! branches and `CALL { … }` bodies start empty, and subquery expressions see
//! the outer variables without changing them.

use std::borrow::Cow;
use std::collections::HashMap;

use super::Catalog;
use super::catalog::{Signature, qualified_name};
use super::structure::StructuralError;
use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::names::Variable;
use crate::ast::pattern::*;
use crate::ast::procedure::{InQueryCall, ProcedureInvocation, StandaloneCall};
use crate::ast::query::*;
use crate::ast::visit::{
    Visit, walk_call_subquery, walk_create, walk_delete, walk_exists_inner, walk_expression,
    walk_filter_expression, walk_foreach, walk_in_query_call, walk_list_comprehension, walk_match,
    walk_merge, walk_pattern_comprehension, walk_reduce_expression, walk_standalone_call,
    walk_unwind,
};
use crate::error::{Span, ValueTypes, VariableKind};

/// What is known statically about a value: the types it can have and, for a
/// list, the types of its elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ty {
    top: ValueTypes,
    element: ValueTypes,
}

impl Ty {
    const ANY: Ty = Ty::of(ValueTypes::ANY);

    const fn of(top: ValueTypes) -> Ty {
        Ty {
            top,
            element: ValueTypes::ANY,
        }
    }

    const fn list_of(element: ValueTypes) -> Ty {
        Ty {
            top: ValueTypes::LIST,
            element,
        }
    }

    /// The type of an element, when this is a list.
    fn element(self) -> Ty {
        if self.top == ValueTypes::LIST {
            Ty::of(self.element)
        } else {
            Ty::ANY
        }
    }

    /// Whether every type this value can have is in `types`.
    fn within(self, types: ValueTypes) -> bool {
        self.top != ValueTypes::ANY && types.contains(self.top)
    }

    /// The entity kind, for conflict messages.
    fn kind(self) -> VariableKind {
        match self.top {
            ValueTypes::NODE => VariableKind::Node,
            ValueTypes::RELATIONSHIP => VariableKind::Relationship,
            ValueTypes::PATH => VariableKind::Path,
            ValueTypes::LIST if self.element == ValueTypes::RELATIONSHIP => {
                VariableKind::Relationships
            }
            ValueTypes::LIST => VariableKind::List,
            _ => VariableKind::Value,
        }
    }
}

const NODE: Ty = Ty::of(ValueTypes::NODE);
const RELATIONSHIP: Ty = Ty::of(ValueTypes::RELATIONSHIP);
const RELATIONSHIPS: Ty = Ty::list_of(ValueTypes::RELATIONSHIP);
const PATH: Ty = Ty::of(ValueTypes::PATH);

/// The types of the visible variables; a missing one is not known.
type Vars = HashMap<String, Ty>;

/// The type errors in `query`, checking calls against `catalog`.
pub(super) fn type_errors(query: &Query, catalog: &Catalog) -> Vec<StructuralError> {
    let mut checker = TypeChecker {
        catalog,
        vars: Vars::new(),
        next_start: Vars::new(),
        errors: Vec::new(),
    };
    checker.visit_query(query);
    checker.errors
}

struct TypeChecker<'a> {
    catalog: &'a Catalog,
    /// The types of the visible variables.
    vars: Vars,
    /// The variables a nested query starts with: none for a `CALL { … }`
    /// body, the outer ones for `EXISTS`, `COUNT` and `COLLECT`.
    next_start: Vars,
    errors: Vec<StructuralError>,
}

impl<'a> TypeChecker<'a> {
    /// Records that `var` is used as `used`, reporting a conflict with an
    /// earlier use.
    fn bind(&mut self, var: &Variable, used: Ty) {
        let name = &var.name.name;
        let bound = self.vars.get(name).copied().unwrap_or(Ty::ANY);
        let fits = bound.top.intersects(used.top)
            && (used.top != ValueTypes::LIST || bound.element.intersects(used.element));
        if !fits {
            self.errors.push(StructuralError::VariableTypeConflict {
                name: name.clone(),
                bound_as: bound.kind(),
                used_as: used.kind(),
                span: var.name.span,
            });
        } else if bound == Ty::ANY {
            // A known type stays: a list reused by `-[rs*]-` is still a list.
            self.vars.insert(name.clone(), used);
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
            self.vars.entry(var.name.name.clone()).or_insert(PATH);
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
            self.bind(var, NODE);
        }
    }

    fn bind_relationship(&mut self, rel: &RelationshipPattern) {
        let Some(detail) = &rel.detail else { return };
        let Some(var) = &detail.variable else { return };
        let ty = if detail.range.is_some() || rel.quantifier.is_some() {
            RELATIONSHIPS
        } else {
            RELATIONSHIP
        };
        self.bind(var, ty);
    }

    /// Runs `visit` with the current variables, discarding what it binds.
    fn nested(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.vars.clone();
        visit(self);
        self.vars = outer;
    }

    /// Replaces the visible variables with the columns of a projection.
    fn project(&mut self, star: bool, items: &[ProjectionItem]) {
        let mut projected = if star { self.vars.clone() } else { Vars::new() };
        for item in items {
            let name = match (&item.alias, &item.expression) {
                (Some(alias), _) => alias.name.name.clone(),
                (None, Expression::Variable(var)) => var.name.name.clone(),
                _ => continue,
            };
            projected.insert(name, self.type_of(&item.expression));
        }
        self.vars = projected;
    }

    /// Visits a `WITH` or `RETURN`. Its `ORDER BY` and `WHERE` see the
    /// columns, which hide the variables read before
    /// (`RETURN n.num AS n ORDER BY n + 2`), as well as those variables.
    fn visit_projection(
        &mut self,
        star: bool,
        items: &[ProjectionItem],
        order: Option<&Order>,
        skip_limit: [&Option<Expression>; 2],
        predicate: Option<&Expression>,
    ) {
        for item in items {
            self.visit_projection_item(item);
        }
        for expr in skip_limit.into_iter().flatten() {
            self.visit_expression(expr);
        }
        let before = self.vars.clone();
        self.project(star, items);
        let columns = std::mem::replace(&mut self.vars, before);
        self.vars.extend(columns.clone());
        if let Some(order) = order {
            self.visit_order(order);
        }
        if let Some(predicate) = predicate {
            self.expect_predicate(predicate, "WHERE");
            self.visit_expression(predicate);
        }
        self.vars = columns;
    }

    /// Binds `var` to `ty` in the current scope, hiding an outer variable.
    fn shadow(&mut self, var: &Variable, ty: Ty) {
        self.vars.insert(var.name.name.clone(), ty);
    }

    /// Reports `expr` unless it can have one of the `expected` types.
    fn expect(&mut self, expr: &Expression, expected: ValueTypes, context: Cow<'static, str>) {
        let found = self.type_of(expr).top;
        if !found.intersects(expected) {
            self.errors.push(StructuralError::InvalidArgumentType {
                context,
                expected,
                found,
                span: expr.span(),
            });
        }
    }

    fn expect_predicate(&mut self, expr: &Expression, context: &'static str) {
        self.expect(expr, ValueTypes::BOOLEAN, Cow::Borrowed(context));
    }

    /// Checks the arguments of a call to `display` against `signature`.
    fn check_arguments(
        &mut self,
        display: String,
        signature: &Signature,
        arguments: &[Expression],
        span: Span,
    ) {
        let found = arguments.len();
        let (min, max) = (signature.min_args(), signature.max_args());
        if found < min || max.is_some_and(|max| found > max) {
            let narrow = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
            self.errors.push(StructuralError::InvalidNumberOfArguments {
                name: display,
                min: narrow(min),
                max: max.map(narrow),
                found: narrow(found),
                span,
            });
            return;
        }
        for (index, argument) in arguments.iter().enumerate() {
            self.expect(
                argument,
                signature.param(index),
                Cow::Owned(format!("argument {} of `{display}`", index + 1)),
            );
        }
    }

    fn check_function_call(&mut self, call: &FunctionInvocation) {
        let name = qualified_name(call);
        let catalog = self.catalog;
        match catalog.function(&name) {
            Some(signature) => {
                self.check_arguments(format!("{name}()"), signature, &call.arguments, call.span)
            }
            None if catalog.is_strict() => self.errors.push(StructuralError::UnknownFunction {
                name,
                span: call.span,
            }),
            None => {}
        }
    }

    fn check_procedure_call(&mut self, call: &ProcedureInvocation) {
        let name = qualified_name(&call.name);
        let catalog = self.catalog;
        match catalog.procedure(&name) {
            // Implicit arguments come from parameters of the same names.
            Some(_) if call.implicit_arguments => {}
            Some(signature) => {
                self.check_arguments(name, signature, &call.name.arguments, call.span)
            }
            None if catalog.is_strict() => self.errors.push(StructuralError::ProcedureNotFound {
                name,
                span: call.span,
            }),
            None => {}
        }
    }

    /// The static type of `expr`.
    fn type_of(&self, expr: &Expression) -> Ty {
        use ValueTypes as T;
        match expr {
            Expression::Literal(literal) => match literal {
                Literal::Number(NumberLiteral::Integer(_)) => Ty::of(T::INTEGER),
                Literal::Number(NumberLiteral::Float(_)) => Ty::of(T::FLOAT),
                Literal::String(_) => Ty::of(T::STRING),
                Literal::Boolean(_) => Ty::of(T::BOOLEAN),
                Literal::Null => Ty::ANY,
                Literal::List(list) => Ty::list_of(
                    list.elements
                        .iter()
                        .map(|e| self.type_of(e).top)
                        .reduce(|a, b| a | b)
                        .unwrap_or(T::ANY),
                ),
                Literal::Map(_) => Ty::of(T::MAP),
            },
            Expression::Variable(var) => self.vars.get(&var.name.name).copied().unwrap_or(Ty::ANY),
            Expression::Parenthesized(inner) => self.type_of(inner),
            Expression::BinaryOp { op, lhs, rhs, .. } => {
                let (lhs, rhs) = (self.type_of(lhs), self.type_of(rhs));
                match op {
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor => {
                        Ty::of(T::BOOLEAN)
                    }
                    _ if lhs.within(T::INTEGER) && rhs.within(T::INTEGER) => {
                        Ty::of(if *op == BinaryOperator::Power {
                            T::FLOAT
                        } else {
                            T::INTEGER
                        })
                    }
                    _ if lhs.within(T::NUMBER) && rhs.within(T::NUMBER) => Ty::of(T::NUMBER),
                    BinaryOperator::Add if lhs.within(T::STRING) && rhs.within(T::STRING) => {
                        Ty::of(T::STRING)
                    }
                    _ => Ty::ANY,
                }
            }
            Expression::UnaryOp { op, operand, .. } => match op {
                UnaryOperator::Not => Ty::of(T::BOOLEAN),
                UnaryOperator::Negate | UnaryOperator::Plus => {
                    let operand = self.type_of(operand);
                    if operand.within(T::NUMBER | T::DURATION) {
                        operand
                    } else {
                        Ty::ANY
                    }
                }
            },
            Expression::NodeLabels { .. }
            | Expression::Comparison { .. }
            | Expression::In { .. }
            | Expression::IsNull { .. }
            | Expression::IsNormalized { .. }
            | Expression::IsTyped { .. }
            | Expression::All(_)
            | Expression::Any(_)
            | Expression::None(_)
            | Expression::Single(_)
            | Expression::Pattern(_)
            | Expression::Exists(_) => Ty::of(T::BOOLEAN),
            Expression::ListIndex { list, .. } => self.type_of(list).element(),
            Expression::ListSlice { list, .. } => {
                let list = self.type_of(list);
                if list.top == T::LIST { list } else { Ty::ANY }
            }
            Expression::FunctionCall(call) => self.result_of(call),
            Expression::CountStar { .. } | Expression::CountSubquery(_) => Ty::of(T::INTEGER),
            Expression::ListComprehension(lc) if lc.map.is_none() => {
                Ty::list_of(self.type_of(&lc.collection).element().top)
            }
            Expression::ListComprehension(_)
            | Expression::PatternComprehension(_)
            | Expression::CollectSubquery(_) => Ty::list_of(T::ANY),
            Expression::MapProjection(_) => Ty::of(T::MAP),
            Expression::Parameter(_)
            | Expression::PropertyLookup { .. }
            | Expression::Case(_)
            | Expression::Reduce(_) => Ty::ANY,
        }
    }

    /// The static result type of a function call.
    fn result_of(&self, call: &FunctionInvocation) -> Ty {
        let name = qualified_name(call);
        let Some(signature) = self.catalog.function(&name) else {
            return Ty::ANY;
        };
        let argument = || {
            call.arguments
                .first()
                .map_or(Ty::ANY, |argument| self.type_of(argument))
        };
        match name.to_ascii_lowercase().as_str() {
            "head" | "last" => argument().element(),
            "tail" => match argument() {
                list if list.top == ValueTypes::LIST => list,
                _ => Ty::of(ValueTypes::LIST),
            },
            "collect" => Ty::list_of(argument().top),
            _ => Ty {
                top: signature.result(),
                element: signature.result_element(),
            },
        }
    }

    /// `+` adds numbers, temporal values and durations, concatenates
    /// strings and lists, and appends to or prepends to a list, so only
    /// operands that are never addable are reported, unless the other side
    /// may be a list (`[1] + true`).
    fn check_addition(&mut self, lhs: &Expression, rhs: &Expression) {
        use ValueTypes as T;
        let never_addable = T::BOOLEAN | T::MAP | T::NODE | T::RELATIONSHIP | T::PATH | T::POINT;
        let addable = T::NUMBER | T::STRING | T::LIST | T::TEMPORAL | T::DURATION;
        let (lhs_ty, rhs_ty) = (self.type_of(lhs), self.type_of(rhs));
        for (operand, ty, other) in [(lhs, lhs_ty, rhs_ty), (rhs, rhs_ty, lhs_ty)] {
            if ty.within(never_addable) && !other.top.intersects(T::LIST) {
                self.errors.push(StructuralError::InvalidArgumentType {
                    context: Cow::Borrowed("+"),
                    expected: addable,
                    found: ty.top,
                    span: operand.span(),
                });
            }
        }
    }

    /// Reports operands of a type their operator does not accept.
    fn check_operands(&mut self, expr: &Expression) {
        use ValueTypes as T;
        let arithmetic = T::NUMBER | T::TEMPORAL | T::DURATION;
        match expr {
            Expression::BinaryOp { op, lhs, rhs, .. } => {
                let (expected, symbol) = match op {
                    BinaryOperator::And => (T::BOOLEAN, "AND"),
                    BinaryOperator::Or => (T::BOOLEAN, "OR"),
                    BinaryOperator::Xor => (T::BOOLEAN, "XOR"),
                    BinaryOperator::Modulo => (T::NUMBER, "%"),
                    BinaryOperator::Power => (T::NUMBER, "^"),
                    BinaryOperator::Subtract => (arithmetic, "-"),
                    BinaryOperator::Multiply => (arithmetic, "*"),
                    BinaryOperator::Divide => (arithmetic, "/"),
                    BinaryOperator::Add => return self.check_addition(lhs, rhs),
                };
                self.expect(lhs, expected, Cow::Borrowed(symbol));
                self.expect(rhs, expected, Cow::Borrowed(symbol));
            }
            Expression::UnaryOp { op, operand, .. } => match op {
                UnaryOperator::Not => self.expect(operand, T::BOOLEAN, Cow::Borrowed("NOT")),
                UnaryOperator::Negate => {
                    self.expect(operand, T::NUMBER | T::DURATION, Cow::Borrowed("unary -"))
                }
                UnaryOperator::Plus => {}
            },
            Expression::In { rhs, .. } => self.expect(rhs, T::LIST, Cow::Borrowed("IN")),
            Expression::PropertyLookup { base, .. } => self.expect(
                base,
                T::NODE | T::RELATIONSHIP | T::MAP | T::POINT | T::TEMPORAL | T::DURATION,
                Cow::Borrowed("property access"),
            ),
            Expression::ListIndex { list, .. } => self.expect(
                list,
                T::LIST | T::MAP | T::NODE | T::RELATIONSHIP,
                Cow::Borrowed("indexing"),
            ),
            Expression::FunctionCall(call) => self.check_function_call(call),
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for TypeChecker<'_> {
    fn visit_query(&mut self, node: &'ast Query) {
        for stmt in &node.statements {
            self.vars.clear();
            self.next_start.clear();
            match stmt {
                QueryBody::SingleQuery(sq) => self.visit_single_query(sq),
                QueryBody::Regular(rq) => self.visit_regular_query(rq),
                QueryBody::Standalone(call) => self.visit_standalone_call(call),
                _ => {}
            }
        }
    }

    fn visit_regular_query(&mut self, node: &'ast RegularQuery) {
        let start = std::mem::take(&mut self.next_start);
        let outer = std::mem::replace(&mut self.vars, start.clone());
        self.visit_single_query(&node.single_query);
        // The columns of the first branch, for `CALL { … }`.
        let columns = std::mem::take(&mut self.vars);
        for union in &node.unions {
            self.vars = start.clone();
            self.visit_single_query(&union.single_query);
        }
        self.vars = outer;
        self.next_start = columns;
    }

    fn visit_match(&mut self, node: &'ast Match) {
        self.bind_pattern(&node.pattern);
        if let Some(predicate) = &node.where_clause {
            self.expect_predicate(predicate, "WHERE");
        }
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

    fn visit_delete(&mut self, node: &'ast Delete) {
        for target in &node.targets {
            // `DELETE n:Label` is reported as an invalid target already.
            if !matches!(target, Expression::NodeLabels { .. }) {
                self.expect(
                    target,
                    ValueTypes::NODE
                        | ValueTypes::RELATIONSHIP
                        | ValueTypes::PATH
                        | ValueTypes::LIST,
                    Cow::Borrowed("DELETE"),
                );
            }
        }
        walk_delete(self, node);
    }

    fn visit_unwind(&mut self, node: &'ast Unwind) {
        walk_unwind(self, node);
        let element = self.type_of(&node.expression).element();
        self.shadow(&node.variable, element);
    }

    fn visit_with(&mut self, node: &'ast With) {
        self.visit_projection(
            node.star,
            &node.items,
            node.order.as_ref(),
            [&node.skip, &node.limit],
            node.where_clause.as_ref(),
        );
    }

    fn visit_return(&mut self, node: &'ast Return) {
        self.visit_projection(
            node.star,
            &node.items,
            node.order.as_ref(),
            [&node.skip, &node.limit],
            None,
        );
    }

    fn visit_in_query_call(&mut self, node: &'ast InQueryCall) {
        self.check_procedure_call(&node.call);
        walk_in_query_call(self, node);
    }

    fn visit_standalone_call(&mut self, node: &'ast StandaloneCall) {
        self.check_procedure_call(&node.call);
        walk_standalone_call(self, node);
    }

    fn visit_call_subquery(&mut self, node: &'ast CallSubquery) {
        self.next_start = Vars::new();
        walk_call_subquery(self, node);
        let columns = std::mem::take(&mut self.next_start);
        if returns(&node.query) {
            self.vars.extend(columns);
        }
    }

    fn visit_exists_inner(&mut self, node: &'ast ExistsInner) {
        self.nested(|this| {
            match node {
                ExistsInner::Pattern(pattern, _) => this.bind_pattern(pattern),
                ExistsInner::RegularQuery(_) => this.next_start = this.vars.clone(),
            }
            walk_exists_inner(this, node);
        });
    }

    fn visit_collect_subquery_expression(&mut self, node: &'ast CollectSubqueryExpression) {
        self.nested(|this| {
            this.next_start = this.vars.clone();
            this.visit_regular_query(&node.query);
        });
    }

    fn visit_expression(&mut self, node: &'ast Expression) {
        self.check_operands(node);
        if let Expression::Pattern(pattern) = node {
            self.nested(|this| this.bind_relationships_pattern(pattern));
        }
        walk_expression(self, node);
    }

    fn visit_pattern_comprehension(&mut self, node: &'ast PatternComprehension) {
        self.nested(|this| {
            this.bind_relationships_pattern(&node.pattern);
            if let Some(var) = &node.variable {
                this.vars.entry(var.name.name.clone()).or_insert(PATH);
            }
            if let Some(predicate) = &node.where_clause {
                this.expect_predicate(predicate, "WHERE");
            }
            walk_pattern_comprehension(this, node);
        });
    }

    fn visit_list_comprehension(&mut self, node: &'ast ListComprehension) {
        let element = self.type_of(&node.collection).element();
        self.nested(|this| {
            this.shadow(&node.variable, element);
            if let Some(predicate) = &node.filter {
                this.expect_predicate(predicate, "WHERE");
            }
            walk_list_comprehension(this, node);
        });
    }

    fn visit_filter_expression(&mut self, node: &'ast FilterExpression) {
        let element = self.type_of(&node.collection).element();
        self.nested(|this| {
            this.shadow(&node.variable, element);
            if let Some(predicate) = &node.predicate {
                this.expect_predicate(predicate, "WHERE");
            }
            walk_filter_expression(this, node);
        });
    }

    fn visit_reduce_expression(&mut self, node: &'ast ReduceExpression) {
        let element = self.type_of(&node.list).element();
        self.nested(|this| {
            this.shadow(&node.accumulator, Ty::ANY);
            this.shadow(&node.variable, element);
            walk_reduce_expression(this, node);
        });
    }

    fn visit_foreach(&mut self, node: &'ast Foreach) {
        let element = self.type_of(&node.list).element();
        self.nested(|this| {
            this.shadow(&node.variable, element);
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
