//! AST visitor traits for openCypher query trees.
//!
//! This module provides two traits for traversing the AST:
//! - [`Visit`] — immutable read-only traversal
//! - [`VisitMut`] — mutable traversal for rewriting
//!
//! Each trait has one method per AST node type (`visit_foo`), with a default
//! implementation that delegates to a free `walk_foo` function. Override only
//! the methods you care about; call the walk function from your override to
//! continue traversal.
//!
//! # Example: counting labels
//! ```
//! use open_cypher::parse;
//! use open_cypher::ast::visit::{Visit, walk_node_pattern};
//! use open_cypher::ast::pattern::NodePattern;
//!
//! struct LabelCounter { count: usize }
//!
//! impl<'ast> Visit<'ast> for LabelCounter {
//!     fn visit_node_pattern(&mut self, node: &'ast NodePattern) {
//!         self.count += node.labels.len();
//!         walk_node_pattern(self, node);
//!     }
//! }
//! ```

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::names::*;
use crate::ast::pattern::*;
use crate::ast::procedure::*;
use crate::ast::query::*;

/// Immutable AST visitor trait.
///
/// Default implementations delegate to free `walk_*` functions for
/// structural traversal. Override only the methods you need.
pub trait Visit<'ast> {
    fn visit_query(&mut self, node: &'ast Query)
    where
        Self: Sized,
    {
        walk_query(self, node)
    }
    fn visit_single_query(&mut self, node: &'ast SingleQuery)
    where
        Self: Sized,
    {
        walk_single_query(self, node)
    }
    fn visit_regular_query(&mut self, node: &'ast RegularQuery)
    where
        Self: Sized,
    {
        walk_regular_query(self, node)
    }
    fn visit_union(&mut self, node: &'ast Union)
    where
        Self: Sized,
    {
        walk_union(self, node)
    }
    fn visit_match(&mut self, node: &'ast Match)
    where
        Self: Sized,
    {
        walk_match(self, node)
    }
    fn visit_create(&mut self, node: &'ast Create)
    where
        Self: Sized,
    {
        walk_create(self, node)
    }
    fn visit_merge(&mut self, node: &'ast Merge)
    where
        Self: Sized,
    {
        walk_merge(self, node)
    }
    fn visit_merge_action(&mut self, node: &'ast MergeAction)
    where
        Self: Sized,
    {
        walk_merge_action(self, node)
    }
    fn visit_delete(&mut self, node: &'ast Delete)
    where
        Self: Sized,
    {
        walk_delete(self, node)
    }
    fn visit_set(&mut self, node: &'ast Set)
    where
        Self: Sized,
    {
        walk_set(self, node)
    }
    fn visit_set_item(&mut self, node: &'ast SetItem)
    where
        Self: Sized,
    {
        walk_set_item(self, node)
    }
    fn visit_remove(&mut self, node: &'ast Remove)
    where
        Self: Sized,
    {
        walk_remove(self, node)
    }
    fn visit_remove_item(&mut self, node: &'ast RemoveItem)
    where
        Self: Sized,
    {
        walk_remove_item(self, node)
    }
    fn visit_with(&mut self, node: &'ast With)
    where
        Self: Sized,
    {
        walk_with(self, node)
    }
    fn visit_return(&mut self, node: &'ast Return)
    where
        Self: Sized,
    {
        walk_return(self, node)
    }
    fn visit_projection_item(&mut self, node: &'ast ProjectionItem)
    where
        Self: Sized,
    {
        walk_projection_item(self, node)
    }
    fn visit_order(&mut self, node: &'ast Order)
    where
        Self: Sized,
    {
        walk_order(self, node)
    }
    fn visit_sort_item(&mut self, node: &'ast SortItem)
    where
        Self: Sized,
    {
        walk_sort_item(self, node)
    }
    fn visit_unwind(&mut self, node: &'ast Unwind)
    where
        Self: Sized,
    {
        walk_unwind(self, node)
    }
    fn visit_standalone_call(&mut self, node: &'ast StandaloneCall)
    where
        Self: Sized,
    {
        walk_standalone_call(self, node)
    }
    fn visit_in_query_call(&mut self, node: &'ast InQueryCall)
    where
        Self: Sized,
    {
        walk_in_query_call(self, node)
    }
    fn visit_procedure_invocation(&mut self, node: &'ast ProcedureInvocation)
    where
        Self: Sized,
    {
        walk_procedure_invocation(self, node)
    }
    fn visit_yield_items(&mut self, node: &'ast YieldItems)
    where
        Self: Sized,
    {
        walk_yield_items(self, node)
    }
    fn visit_yield_item(&mut self, node: &'ast YieldItem)
    where
        Self: Sized,
    {
        walk_yield_item(self, node)
    }
    fn visit_pattern(&mut self, node: &'ast Pattern)
    where
        Self: Sized,
    {
        walk_pattern(self, node)
    }
    fn visit_pattern_part(&mut self, node: &'ast PatternPart)
    where
        Self: Sized,
    {
        walk_pattern_part(self, node)
    }
    fn visit_anonymous_pattern_part(&mut self, node: &'ast AnonymousPatternPart)
    where
        Self: Sized,
    {
        walk_anonymous_pattern_part(self, node)
    }
    fn visit_pattern_element(&mut self, node: &'ast PatternElement)
    where
        Self: Sized,
    {
        walk_pattern_element(self, node)
    }
    fn visit_node_pattern(&mut self, node: &'ast NodePattern)
    where
        Self: Sized,
    {
        walk_node_pattern(self, node)
    }
    fn visit_pattern_element_chain(&mut self, node: &'ast PatternElementChain)
    where
        Self: Sized,
    {
        walk_pattern_element_chain(self, node)
    }
    fn visit_relationship_pattern(&mut self, node: &'ast RelationshipPattern)
    where
        Self: Sized,
    {
        walk_relationship_pattern(self, node)
    }
    fn visit_relationship_detail(&mut self, node: &'ast RelationshipDetail)
    where
        Self: Sized,
    {
        walk_relationship_detail(self, node)
    }
    fn visit_range_literal(&mut self, node: &'ast RangeLiteral)
    where
        Self: Sized,
    {
        walk_range_literal(self, node)
    }
    fn visit_relationships_pattern(&mut self, node: &'ast RelationshipsPattern)
    where
        Self: Sized,
    {
        walk_relationships_pattern(self, node)
    }
    fn visit_expression(&mut self, node: &'ast Expression)
    where
        Self: Sized,
    {
        walk_expression(self, node)
    }
    fn visit_literal(&mut self, node: &'ast Literal)
    where
        Self: Sized,
    {
        walk_literal(self, node)
    }
    fn visit_number_literal(&mut self, node: &'ast NumberLiteral)
    where
        Self: Sized,
    {
        walk_number_literal(self, node)
    }
    fn visit_string_literal(&mut self, node: &'ast StringLiteral)
    where
        Self: Sized,
    {
        walk_string_literal(self, node)
    }
    fn visit_list_literal(&mut self, node: &'ast ListLiteral)
    where
        Self: Sized,
    {
        walk_list_literal(self, node)
    }
    fn visit_map_literal(&mut self, node: &'ast MapLiteral)
    where
        Self: Sized,
    {
        walk_map_literal(self, node)
    }
    fn visit_parameter(&mut self, node: &'ast Parameter)
    where
        Self: Sized,
    {
        walk_parameter(self, node)
    }
    fn visit_function_invocation_expr(&mut self, node: &'ast FunctionInvocation)
    where
        Self: Sized,
    {
        walk_function_invocation_expr(self, node)
    }
    fn visit_case_expression(&mut self, node: &'ast CaseExpression)
    where
        Self: Sized,
    {
        walk_case_expression(self, node)
    }
    fn visit_case_alternative(&mut self, node: &'ast CaseAlternative)
    where
        Self: Sized,
    {
        walk_case_alternative(self, node)
    }
    fn visit_list_comprehension(&mut self, node: &'ast ListComprehension)
    where
        Self: Sized,
    {
        walk_list_comprehension(self, node)
    }
    fn visit_pattern_comprehension(&mut self, node: &'ast PatternComprehension)
    where
        Self: Sized,
    {
        walk_pattern_comprehension(self, node)
    }
    fn visit_filter_expression(&mut self, node: &'ast FilterExpression)
    where
        Self: Sized,
    {
        walk_filter_expression(self, node)
    }
    fn visit_exists_expression(&mut self, node: &'ast ExistsExpression)
    where
        Self: Sized,
    {
        walk_exists_expression(self, node)
    }
    fn visit_exists_inner(&mut self, node: &'ast ExistsInner)
    where
        Self: Sized,
    {
        walk_exists_inner(self, node)
    }
    fn visit_variable(&mut self, node: &'ast Variable)
    where
        Self: Sized,
    {
        walk_variable(self, node)
    }
    fn visit_symbolic_name(&mut self, node: &'ast SymbolicName)
    where
        Self: Sized,
    {
        walk_symbolic_name(self, node)
    }
    fn visit_properties(&mut self, node: &'ast Properties)
    where
        Self: Sized,
    {
        walk_properties(self, node)
    }
    fn visit_set_operator(&mut self, _node: &'ast SetOperator) {}
    fn visit_sort_direction(&mut self, _node: &'ast SortDirection) {}
    fn visit_relationship_direction(&mut self, _node: &'ast RelationshipDirection) {}
    fn visit_binary_operator(&mut self, _node: &'ast BinaryOperator) {}
    fn visit_unary_operator(&mut self, _node: &'ast UnaryOperator) {}
    fn visit_comparison_operator(&mut self, _node: &'ast ComparisonOperator) {}
    fn visit_rel_type_name(&mut self, node: &'ast RelTypeName)
    where
        Self: Sized,
    {
        walk_rel_type_name(self, node)
    }
    fn visit_property_key_name(&mut self, node: &'ast PropertyKeyName)
    where
        Self: Sized,
    {
        walk_property_key_name(self, node)
    }
}

/// Mutable AST visitor trait for rewriting.
pub trait VisitMut {
    fn visit_query(&mut self, node: &mut Query)
    where
        Self: Sized,
    {
        walk_query_mut(self, node)
    }
    fn visit_single_query(&mut self, node: &mut SingleQuery)
    where
        Self: Sized,
    {
        walk_single_query_mut(self, node)
    }
    fn visit_regular_query(&mut self, node: &mut RegularQuery)
    where
        Self: Sized,
    {
        walk_regular_query_mut(self, node)
    }
    fn visit_union(&mut self, node: &mut Union)
    where
        Self: Sized,
    {
        walk_union_mut(self, node)
    }
    fn visit_match(&mut self, node: &mut Match)
    where
        Self: Sized,
    {
        walk_match_mut(self, node)
    }
    fn visit_create(&mut self, node: &mut Create)
    where
        Self: Sized,
    {
        walk_create_mut(self, node)
    }
    fn visit_merge(&mut self, node: &mut Merge)
    where
        Self: Sized,
    {
        walk_merge_mut(self, node)
    }
    fn visit_merge_action(&mut self, node: &mut MergeAction)
    where
        Self: Sized,
    {
        walk_merge_action_mut(self, node)
    }
    fn visit_delete(&mut self, node: &mut Delete)
    where
        Self: Sized,
    {
        walk_delete_mut(self, node)
    }
    fn visit_set(&mut self, node: &mut Set)
    where
        Self: Sized,
    {
        walk_set_mut(self, node)
    }
    fn visit_set_item(&mut self, node: &mut SetItem)
    where
        Self: Sized,
    {
        walk_set_item_mut(self, node)
    }
    fn visit_remove(&mut self, node: &mut Remove)
    where
        Self: Sized,
    {
        walk_remove_mut(self, node)
    }
    fn visit_remove_item(&mut self, node: &mut RemoveItem)
    where
        Self: Sized,
    {
        walk_remove_item_mut(self, node)
    }
    fn visit_with(&mut self, node: &mut With)
    where
        Self: Sized,
    {
        walk_with_mut(self, node)
    }
    fn visit_return(&mut self, node: &mut Return)
    where
        Self: Sized,
    {
        walk_return_mut(self, node)
    }
    fn visit_projection_item(&mut self, node: &mut ProjectionItem)
    where
        Self: Sized,
    {
        walk_projection_item_mut(self, node)
    }
    fn visit_order(&mut self, node: &mut Order)
    where
        Self: Sized,
    {
        walk_order_mut(self, node)
    }
    fn visit_sort_item(&mut self, node: &mut SortItem)
    where
        Self: Sized,
    {
        walk_sort_item_mut(self, node)
    }
    fn visit_unwind(&mut self, node: &mut Unwind)
    where
        Self: Sized,
    {
        walk_unwind_mut(self, node)
    }
    fn visit_standalone_call(&mut self, node: &mut StandaloneCall)
    where
        Self: Sized,
    {
        walk_standalone_call_mut(self, node)
    }
    fn visit_in_query_call(&mut self, node: &mut InQueryCall)
    where
        Self: Sized,
    {
        walk_in_query_call_mut(self, node)
    }
    fn visit_procedure_invocation(&mut self, node: &mut ProcedureInvocation)
    where
        Self: Sized,
    {
        walk_procedure_invocation_mut(self, node)
    }
    fn visit_yield_items(&mut self, node: &mut YieldItems)
    where
        Self: Sized,
    {
        walk_yield_items_mut(self, node)
    }
    fn visit_yield_item(&mut self, node: &mut YieldItem)
    where
        Self: Sized,
    {
        walk_yield_item_mut(self, node)
    }
    fn visit_pattern(&mut self, node: &mut Pattern)
    where
        Self: Sized,
    {
        walk_pattern_mut(self, node)
    }
    fn visit_pattern_part(&mut self, node: &mut PatternPart)
    where
        Self: Sized,
    {
        walk_pattern_part_mut(self, node)
    }
    fn visit_anonymous_pattern_part(&mut self, node: &mut AnonymousPatternPart)
    where
        Self: Sized,
    {
        walk_anonymous_pattern_part_mut(self, node)
    }
    fn visit_pattern_element(&mut self, node: &mut PatternElement)
    where
        Self: Sized,
    {
        walk_pattern_element_mut(self, node)
    }
    fn visit_node_pattern(&mut self, node: &mut NodePattern)
    where
        Self: Sized,
    {
        walk_node_pattern_mut(self, node)
    }
    fn visit_pattern_element_chain(&mut self, node: &mut PatternElementChain)
    where
        Self: Sized,
    {
        walk_pattern_element_chain_mut(self, node)
    }
    fn visit_relationship_pattern(&mut self, node: &mut RelationshipPattern)
    where
        Self: Sized,
    {
        walk_relationship_pattern_mut(self, node)
    }
    fn visit_relationship_detail(&mut self, node: &mut RelationshipDetail)
    where
        Self: Sized,
    {
        walk_relationship_detail_mut(self, node)
    }
    fn visit_range_literal(&mut self, node: &mut RangeLiteral)
    where
        Self: Sized,
    {
        walk_range_literal_mut(self, node)
    }
    fn visit_relationships_pattern(&mut self, node: &mut RelationshipsPattern)
    where
        Self: Sized,
    {
        walk_relationships_pattern_mut(self, node)
    }
    fn visit_expression(&mut self, node: &mut Expression)
    where
        Self: Sized,
    {
        walk_expression_mut(self, node)
    }
    fn visit_literal(&mut self, node: &mut Literal)
    where
        Self: Sized,
    {
        walk_literal_mut(self, node)
    }
    fn visit_number_literal(&mut self, node: &mut NumberLiteral)
    where
        Self: Sized,
    {
        walk_number_literal_mut(self, node)
    }
    fn visit_string_literal(&mut self, node: &mut StringLiteral)
    where
        Self: Sized,
    {
        walk_string_literal_mut(self, node)
    }
    fn visit_list_literal(&mut self, node: &mut ListLiteral)
    where
        Self: Sized,
    {
        walk_list_literal_mut(self, node)
    }
    fn visit_map_literal(&mut self, node: &mut MapLiteral)
    where
        Self: Sized,
    {
        walk_map_literal_mut(self, node)
    }
    fn visit_parameter(&mut self, node: &mut Parameter)
    where
        Self: Sized,
    {
        walk_parameter_mut(self, node)
    }
    fn visit_function_invocation_expr(&mut self, node: &mut FunctionInvocation)
    where
        Self: Sized,
    {
        walk_function_invocation_expr_mut(self, node)
    }
    fn visit_case_expression(&mut self, node: &mut CaseExpression)
    where
        Self: Sized,
    {
        walk_case_expression_mut(self, node)
    }
    fn visit_case_alternative(&mut self, node: &mut CaseAlternative)
    where
        Self: Sized,
    {
        walk_case_alternative_mut(self, node)
    }
    fn visit_list_comprehension(&mut self, node: &mut ListComprehension)
    where
        Self: Sized,
    {
        walk_list_comprehension_mut(self, node)
    }
    fn visit_pattern_comprehension(&mut self, node: &mut PatternComprehension)
    where
        Self: Sized,
    {
        walk_pattern_comprehension_mut(self, node)
    }
    fn visit_filter_expression(&mut self, node: &mut FilterExpression)
    where
        Self: Sized,
    {
        walk_filter_expression_mut(self, node)
    }
    fn visit_exists_expression(&mut self, node: &mut ExistsExpression)
    where
        Self: Sized,
    {
        walk_exists_expression_mut(self, node)
    }
    fn visit_exists_inner(&mut self, node: &mut ExistsInner)
    where
        Self: Sized,
    {
        walk_exists_inner_mut(self, node)
    }
    fn visit_variable(&mut self, node: &mut Variable)
    where
        Self: Sized,
    {
        walk_variable_mut(self, node)
    }
    fn visit_symbolic_name(&mut self, node: &mut SymbolicName)
    where
        Self: Sized,
    {
        walk_symbolic_name_mut(self, node)
    }
    fn visit_properties(&mut self, node: &mut Properties)
    where
        Self: Sized,
    {
        walk_properties_mut(self, node)
    }
    fn visit_rel_type_name(&mut self, node: &mut RelTypeName)
    where
        Self: Sized,
    {
        walk_rel_type_name_mut(self, node)
    }
    fn visit_property_key_name(&mut self, node: &mut PropertyKeyName)
    where
        Self: Sized,
    {
        walk_property_key_name_mut(self, node)
    }
}

// ── Free walk functions (immutable) ─────────────────────────────────

pub fn walk_query<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Query) {
    for stmt in &node.statements {
        match stmt {
            QueryBody::SingleQuery(sq) => v.visit_single_query(sq),
            QueryBody::Standalone(sc) => v.visit_standalone_call(sc),
        }
    }
}

pub fn walk_single_query<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast SingleQuery) {
    match &node.kind {
        SingleQueryKind::SinglePart(sp) => {
            for rc in &sp.reading_clauses {
                match rc {
                    ReadingClause::Match(m) => v.visit_match(m),
                    ReadingClause::Unwind(u) => v.visit_unwind(u),
                    ReadingClause::InQueryCall(i) => v.visit_in_query_call(i),
                }
            }
            match &sp.body {
                SinglePartBody::Return(ret) => v.visit_return(ret),
                SinglePartBody::Updating {
                    updating,
                    return_clause,
                } => {
                    for uc in updating {
                        match uc {
                            UpdatingClause::Create(c) => v.visit_create(c),
                            UpdatingClause::Merge(m) => v.visit_merge(m),
                            UpdatingClause::Delete(d) => v.visit_delete(d),
                            UpdatingClause::Set(s) => v.visit_set(s),
                            UpdatingClause::Remove(r) => v.visit_remove(r),
                        }
                    }
                    if let Some(ret) = return_clause {
                        v.visit_return(ret);
                    }
                }
            }
        }
        SingleQueryKind::MultiPart(mp) => {
            for part in &mp.parts {
                for rc in &part.reading_clauses {
                    match rc {
                        ReadingClause::Match(m) => v.visit_match(m),
                        ReadingClause::Unwind(u) => v.visit_unwind(u),
                        ReadingClause::InQueryCall(i) => v.visit_in_query_call(i),
                    }
                }
                for uc in &part.updating_clauses {
                    match uc {
                        UpdatingClause::Create(c) => v.visit_create(c),
                        UpdatingClause::Merge(m) => v.visit_merge(m),
                        UpdatingClause::Delete(d) => v.visit_delete(d),
                        UpdatingClause::Set(s) => v.visit_set(s),
                        UpdatingClause::Remove(r) => v.visit_remove(r),
                    }
                }
                v.visit_with(&part.with);
            }
            // Visit the final_part's reading clauses and body directly
            for rc in &mp.final_part.reading_clauses {
                match rc {
                    ReadingClause::Match(m) => v.visit_match(m),
                    ReadingClause::Unwind(u) => v.visit_unwind(u),
                    ReadingClause::InQueryCall(i) => v.visit_in_query_call(i),
                }
            }
            match &mp.final_part.body {
                SinglePartBody::Return(ret) => v.visit_return(ret),
                SinglePartBody::Updating {
                    updating,
                    return_clause,
                } => {
                    for uc in updating {
                        match uc {
                            UpdatingClause::Create(c) => v.visit_create(c),
                            UpdatingClause::Merge(m) => v.visit_merge(m),
                            UpdatingClause::Delete(d) => v.visit_delete(d),
                            UpdatingClause::Set(s) => v.visit_set(s),
                            UpdatingClause::Remove(r) => v.visit_remove(r),
                        }
                    }
                    if let Some(ret) = return_clause {
                        v.visit_return(ret);
                    }
                }
            }
        }
    }
}

pub fn walk_regular_query<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast RegularQuery) {
    v.visit_single_query(&node.single_query);
    for u in &node.unions {
        v.visit_union(u);
    }
}

pub fn walk_union<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Union) {
    v.visit_single_query(&node.single_query);
}

pub fn walk_match<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Match) {
    v.visit_pattern(&node.pattern);
    if let Some(expr) = &node.where_clause {
        v.visit_expression(expr);
    }
}

pub fn walk_create<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Create) {
    v.visit_pattern(&node.pattern);
}

pub fn walk_merge<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Merge) {
    v.visit_pattern_part(&node.pattern);
    for a in &node.actions {
        v.visit_merge_action(a);
    }
}

pub fn walk_merge_action<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast MergeAction) {
    for s in &node.set_items {
        v.visit_set_item(s);
    }
}

pub fn walk_delete<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Delete) {
    for t in &node.targets {
        v.visit_expression(t);
    }
}

pub fn walk_set<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Set) {
    for s in &node.items {
        v.visit_set_item(s);
    }
}

pub fn walk_set_item<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast SetItem) {
    match node {
        SetItem::Property { property, value } => {
            v.visit_expression(property);
            v.visit_expression(value);
        }
        SetItem::Variable {
            variable,
            value,
            operator,
        } => {
            v.visit_variable(variable);
            v.visit_expression(value);
            v.visit_set_operator(operator);
        }
        SetItem::Labels { variable, labels } => {
            v.visit_variable(variable);
            for l in labels {
                v.visit_symbolic_name(l);
            }
        }
    }
}

pub fn walk_remove<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Remove) {
    for r in &node.items {
        v.visit_remove_item(r);
    }
}

pub fn walk_remove_item<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast RemoveItem) {
    match node {
        RemoveItem::Labels { variable, labels } => {
            v.visit_variable(variable);
            for l in labels {
                v.visit_symbolic_name(l);
            }
        }
        RemoveItem::Property(expr) => v.visit_expression(expr),
    }
}

pub fn walk_with<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast With) {
    for p in &node.items {
        v.visit_projection_item(p);
    }
    if let Some(o) = &node.order {
        v.visit_order(o);
    }
    if let Some(e) = &node.skip {
        v.visit_expression(e);
    }
    if let Some(e) = &node.limit {
        v.visit_expression(e);
    }
    if let Some(e) = &node.where_clause {
        v.visit_expression(e);
    }
}

pub fn walk_return<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Return) {
    for p in &node.items {
        v.visit_projection_item(p);
    }
    if let Some(o) = &node.order {
        v.visit_order(o);
    }
    if let Some(e) = &node.skip {
        v.visit_expression(e);
    }
    if let Some(e) = &node.limit {
        v.visit_expression(e);
    }
}

pub fn walk_projection_item<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast ProjectionItem) {
    v.visit_expression(&node.expression);
    if let Some(var) = &node.alias {
        v.visit_variable(var);
    }
}

pub fn walk_order<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Order) {
    for s in &node.items {
        v.visit_sort_item(s);
    }
}

pub fn walk_sort_item<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast SortItem) {
    v.visit_expression(&node.expression);
    if let Some(d) = &node.direction {
        v.visit_sort_direction(d);
    }
}

pub fn walk_unwind<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Unwind) {
    v.visit_expression(&node.expression);
    v.visit_variable(&node.variable);
}

pub fn walk_standalone_call<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast StandaloneCall) {
    v.visit_procedure_invocation(&node.call);
    if let Some(y) = &node.yield_items {
        match y {
            YieldSpec::Star { .. } => {}
            YieldSpec::Items(yi) => v.visit_yield_items(yi),
        }
    }
}

pub fn walk_in_query_call<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast InQueryCall) {
    v.visit_procedure_invocation(&node.call);
    if let Some(y) = &node.yield_items {
        v.visit_yield_items(y);
    }
}

pub fn walk_procedure_invocation<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast ProcedureInvocation) {
    v.visit_function_invocation_expr(&node.name);
}

pub fn walk_yield_items<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast YieldItems) {
    for y in &node.items {
        v.visit_yield_item(y);
    }
    if let Some(e) = &node.where_clause {
        v.visit_expression(e);
    }
}

pub fn walk_yield_item<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast YieldItem) {
    v.visit_symbolic_name(&node.procedure_field);
    if let Some(var) = &node.alias {
        v.visit_variable(var);
    }
}

pub fn walk_pattern<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Pattern) {
    for p in &node.parts {
        v.visit_pattern_part(p);
    }
}

pub fn walk_pattern_part<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast PatternPart) {
    if let Some(var) = &node.variable {
        v.visit_variable(var);
    }
    v.visit_anonymous_pattern_part(&node.anonymous);
}

pub fn walk_anonymous_pattern_part<'ast, V: Visit<'ast>>(
    v: &mut V,
    node: &'ast AnonymousPatternPart,
) {
    v.visit_pattern_element(&node.element);
}

pub fn walk_pattern_element<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast PatternElement) {
    match node {
        PatternElement::Path { start, chains } => {
            v.visit_node_pattern(start);
            for c in chains {
                v.visit_pattern_element_chain(c);
            }
        }
        PatternElement::Parenthesized(inner) => {
            v.visit_pattern_element(inner);
        }
    }
}

pub fn walk_node_pattern<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast NodePattern) {
    if let Some(var) = &node.variable {
        v.visit_variable(var);
    }
    for l in &node.labels {
        v.visit_symbolic_name(l);
    }
    if let Some(props) = &node.properties {
        v.visit_properties(props);
    }
}

pub fn walk_pattern_element_chain<'ast, V: Visit<'ast>>(
    v: &mut V,
    node: &'ast PatternElementChain,
) {
    v.visit_relationship_pattern(&node.relationship);
    v.visit_node_pattern(&node.node);
}

pub fn walk_relationship_pattern<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast RelationshipPattern) {
    v.visit_relationship_direction(&node.direction);
    if let Some(d) = &node.detail {
        v.visit_relationship_detail(d);
    }
}

pub fn walk_relationship_detail<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast RelationshipDetail) {
    if let Some(var) = &node.variable {
        v.visit_variable(var);
    }
    for t in &node.types {
        v.visit_rel_type_name(t);
    }
    if let Some(r) = &node.range {
        v.visit_range_literal(r);
    }
    if let Some(props) = &node.properties {
        v.visit_properties(props);
    }
}

pub fn walk_range_literal<'ast, V: Visit<'ast>>(_v: &mut V, _node: &'ast RangeLiteral) {}

pub fn walk_relationships_pattern<'ast, V: Visit<'ast>>(
    v: &mut V,
    node: &'ast RelationshipsPattern,
) {
    v.visit_node_pattern(&node.start);
    for c in &node.chains {
        v.visit_pattern_element_chain(c);
    }
}

pub fn walk_expression<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Expression) {
    match node {
        Expression::Literal(l) => v.visit_literal(l),
        Expression::Variable(var) => v.visit_variable(var),
        Expression::Parameter(p) => v.visit_parameter(p),
        Expression::PropertyLookup { base, property, .. } => {
            v.visit_expression(base);
            v.visit_property_key_name(property);
        }
        Expression::NodeLabels { base, labels, .. } => {
            v.visit_expression(base);
            for l in labels {
                v.visit_symbolic_name(l);
            }
        }
        Expression::BinaryOp { op, lhs, rhs, .. } => {
            v.visit_binary_operator(op);
            v.visit_expression(lhs);
            v.visit_expression(rhs);
        }
        Expression::UnaryOp { op, operand, .. } => {
            v.visit_unary_operator(op);
            v.visit_expression(operand);
        }
        Expression::Comparison { lhs, operators, .. } => {
            v.visit_expression(lhs);
            for (op, rhs) in operators {
                v.visit_comparison_operator(op);
                v.visit_expression(rhs);
            }
        }
        Expression::ListIndex { list, index, .. } => {
            v.visit_expression(list);
            v.visit_expression(index);
        }
        Expression::ListSlice {
            list, start, end, ..
        } => {
            v.visit_expression(list);
            if let Some(s) = start {
                v.visit_expression(s);
            }
            if let Some(e) = end {
                v.visit_expression(e);
            }
        }
        Expression::In { lhs, rhs, .. } => {
            v.visit_expression(lhs);
            v.visit_expression(rhs);
        }
        Expression::IsNull { operand, .. } => {
            v.visit_expression(operand);
        }
        Expression::FunctionCall(func) => v.visit_function_invocation_expr(func),
        Expression::CountStar { .. } => {}
        Expression::Case(case) => v.visit_case_expression(case),
        Expression::ListComprehension(lc) => v.visit_list_comprehension(lc),
        Expression::PatternComprehension(pc) => v.visit_pattern_comprehension(pc),
        Expression::All(fe)
        | Expression::Any(fe)
        | Expression::None(fe)
        | Expression::Single(fe) => v.visit_filter_expression(fe),
        Expression::Parenthesized(inner) => v.visit_expression(inner),
        Expression::Pattern(rp) => v.visit_relationships_pattern(rp),
        Expression::Exists(exists) => v.visit_exists_expression(exists),
    }
}

pub fn walk_literal<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Literal) {
    match node {
        Literal::Number(n) => v.visit_number_literal(n),
        Literal::String(s) => v.visit_string_literal(s),
        Literal::Boolean(_) => {}
        Literal::Null => {}
        Literal::List(l) => v.visit_list_literal(l),
        Literal::Map(m) => v.visit_map_literal(m),
    }
}

pub fn walk_number_literal<'ast, V: Visit<'ast>>(_v: &mut V, _node: &'ast NumberLiteral) {}

pub fn walk_string_literal<'ast, V: Visit<'ast>>(_v: &mut V, node: &'ast StringLiteral) {
    let _ = node.value.as_str();
}

pub fn walk_list_literal<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast ListLiteral) {
    for e in &node.elements {
        v.visit_expression(e);
    }
}

pub fn walk_map_literal<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast MapLiteral) {
    for (key, val) in &node.entries {
        v.visit_property_key_name(key);
        v.visit_expression(val);
    }
}

pub fn walk_parameter<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Parameter) {
    v.visit_symbolic_name(&node.name);
}

pub fn walk_function_invocation_expr<'ast, V: Visit<'ast>>(
    v: &mut V,
    node: &'ast FunctionInvocation,
) {
    for n in &node.name {
        v.visit_symbolic_name(n);
    }
    for a in &node.arguments {
        v.visit_expression(a);
    }
}

pub fn walk_case_expression<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast CaseExpression) {
    if let Some(s) = &node.scrutinee {
        v.visit_expression(s);
    }
    for a in &node.alternatives {
        v.visit_case_alternative(a);
    }
    if let Some(d) = &node.default {
        v.visit_expression(d);
    }
}

pub fn walk_case_alternative<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast CaseAlternative) {
    v.visit_expression(&node.when);
    v.visit_expression(&node.then);
}

pub fn walk_list_comprehension<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast ListComprehension) {
    v.visit_variable(&node.variable);
    if let Some(f) = &node.filter {
        v.visit_expression(f);
    }
    if let Some(m) = &node.map {
        v.visit_expression(m);
    }
}

pub fn walk_pattern_comprehension<'ast, V: Visit<'ast>>(
    v: &mut V,
    node: &'ast PatternComprehension,
) {
    if let Some(var) = &node.variable {
        v.visit_variable(var);
    }
    v.visit_relationships_pattern(&node.pattern);
    if let Some(w) = &node.where_clause {
        v.visit_expression(w);
    }
    v.visit_expression(&node.map);
}

pub fn walk_filter_expression<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast FilterExpression) {
    v.visit_variable(&node.variable);
    v.visit_expression(&node.collection);
    if let Some(p) = &node.predicate {
        v.visit_expression(p);
    }
}

pub fn walk_exists_expression<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast ExistsExpression) {
    v.visit_exists_inner(&node.inner);
}

pub fn walk_exists_inner<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast ExistsInner) {
    match node {
        ExistsInner::Pattern(p, where_clause) => {
            v.visit_pattern(p);
            if let Some(w) = where_clause {
                v.visit_expression(w);
            }
        }
        ExistsInner::RegularQuery(rq) => v.visit_regular_query(rq),
    }
}

pub fn walk_variable<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Variable) {
    v.visit_symbolic_name(&node.name);
}

pub fn walk_symbolic_name<'ast, V: Visit<'ast>>(_v: &mut V, _node: &'ast SymbolicName) {}

pub fn walk_properties<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast Properties) {
    match node {
        Properties::Map(m) => v.visit_map_literal(m),
        Properties::Parameter(p) => v.visit_parameter(p),
    }
}

pub fn walk_rel_type_name<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast RelTypeName) {
    v.visit_symbolic_name(&node.name);
}

pub fn walk_property_key_name<'ast, V: Visit<'ast>>(v: &mut V, node: &'ast PropertyKeyName) {
    v.visit_symbolic_name(&node.name);
}

// ── Free walk functions (mutable) ────────────────────────────────────

pub fn walk_query_mut<V: VisitMut>(v: &mut V, node: &mut Query) {
    for stmt in &mut node.statements {
        match stmt {
            QueryBody::SingleQuery(sq) => v.visit_single_query(sq),
            QueryBody::Standalone(sc) => v.visit_standalone_call(sc),
        }
    }
}

pub fn walk_single_query_mut<V: VisitMut>(v: &mut V, node: &mut SingleQuery) {
    match &mut node.kind {
        SingleQueryKind::SinglePart(sp) => {
            for rc in &mut sp.reading_clauses {
                match rc {
                    ReadingClause::Match(m) => v.visit_match(m),
                    ReadingClause::Unwind(u) => v.visit_unwind(u),
                    ReadingClause::InQueryCall(i) => v.visit_in_query_call(i),
                }
            }
            match &mut sp.body {
                SinglePartBody::Return(ret) => v.visit_return(ret),
                SinglePartBody::Updating {
                    updating,
                    return_clause,
                } => {
                    for uc in updating {
                        match uc {
                            UpdatingClause::Create(c) => v.visit_create(c),
                            UpdatingClause::Merge(m) => v.visit_merge(m),
                            UpdatingClause::Delete(d) => v.visit_delete(d),
                            UpdatingClause::Set(s) => v.visit_set(s),
                            UpdatingClause::Remove(r) => v.visit_remove(r),
                        }
                    }
                    if let Some(ret) = return_clause {
                        v.visit_return(ret);
                    }
                }
            }
        }
        SingleQueryKind::MultiPart(mp) => {
            for part in &mut mp.parts {
                for rc in &mut part.reading_clauses {
                    match rc {
                        ReadingClause::Match(m) => v.visit_match(m),
                        ReadingClause::Unwind(u) => v.visit_unwind(u),
                        ReadingClause::InQueryCall(i) => v.visit_in_query_call(i),
                    }
                }
                for uc in &mut part.updating_clauses {
                    match uc {
                        UpdatingClause::Create(c) => v.visit_create(c),
                        UpdatingClause::Merge(m) => v.visit_merge(m),
                        UpdatingClause::Delete(d) => v.visit_delete(d),
                        UpdatingClause::Set(s) => v.visit_set(s),
                        UpdatingClause::Remove(r) => v.visit_remove(r),
                    }
                }
                v.visit_with(&mut part.with);
            }
            let mut final_sp = SingleQuery {
                kind: SingleQueryKind::SinglePart(mp.final_part.clone()),
            };
            v.visit_single_query(&mut final_sp);
        }
    }
}

pub fn walk_regular_query_mut<V: VisitMut>(v: &mut V, node: &mut RegularQuery) {
    v.visit_single_query(&mut node.single_query);
    for u in &mut node.unions {
        v.visit_union(u);
    }
}

pub fn walk_union_mut<V: VisitMut>(v: &mut V, node: &mut Union) {
    v.visit_single_query(&mut node.single_query);
}

pub fn walk_match_mut<V: VisitMut>(v: &mut V, node: &mut Match) {
    v.visit_pattern(&mut node.pattern);
    if let Some(expr) = &mut node.where_clause {
        v.visit_expression(expr);
    }
}

pub fn walk_create_mut<V: VisitMut>(v: &mut V, node: &mut Create) {
    v.visit_pattern(&mut node.pattern);
}

pub fn walk_merge_mut<V: VisitMut>(v: &mut V, node: &mut Merge) {
    v.visit_pattern_part(&mut node.pattern);
    for a in &mut node.actions {
        v.visit_merge_action(a);
    }
}

pub fn walk_merge_action_mut<V: VisitMut>(v: &mut V, node: &mut MergeAction) {
    for s in &mut node.set_items {
        v.visit_set_item(s);
    }
}

pub fn walk_delete_mut<V: VisitMut>(v: &mut V, node: &mut Delete) {
    for t in &mut node.targets {
        v.visit_expression(t);
    }
}

pub fn walk_set_mut<V: VisitMut>(v: &mut V, node: &mut Set) {
    for s in &mut node.items {
        v.visit_set_item(s);
    }
}

pub fn walk_set_item_mut<V: VisitMut>(v: &mut V, node: &mut SetItem) {
    match node {
        SetItem::Property { property, value } => {
            v.visit_expression(property);
            v.visit_expression(value);
        }
        SetItem::Variable {
            variable, value, ..
        } => {
            v.visit_variable(variable);
            v.visit_expression(value);
        }
        SetItem::Labels { variable, labels } => {
            v.visit_variable(variable);
            for l in labels {
                v.visit_symbolic_name(l);
            }
        }
    }
}

pub fn walk_remove_mut<V: VisitMut>(v: &mut V, node: &mut Remove) {
    for r in &mut node.items {
        v.visit_remove_item(r);
    }
}

pub fn walk_remove_item_mut<V: VisitMut>(v: &mut V, node: &mut RemoveItem) {
    match node {
        RemoveItem::Labels { variable, labels } => {
            v.visit_variable(variable);
            for l in labels {
                v.visit_symbolic_name(l);
            }
        }
        RemoveItem::Property(expr) => v.visit_expression(expr),
    }
}

pub fn walk_with_mut<V: VisitMut>(v: &mut V, node: &mut With) {
    for p in &mut node.items {
        v.visit_projection_item(p);
    }
    if let Some(o) = &mut node.order {
        v.visit_order(o);
    }
    if let Some(e) = &mut node.skip {
        v.visit_expression(e);
    }
    if let Some(e) = &mut node.limit {
        v.visit_expression(e);
    }
    if let Some(e) = &mut node.where_clause {
        v.visit_expression(e);
    }
}

pub fn walk_return_mut<V: VisitMut>(v: &mut V, node: &mut Return) {
    for p in &mut node.items {
        v.visit_projection_item(p);
    }
    if let Some(o) = &mut node.order {
        v.visit_order(o);
    }
    if let Some(e) = &mut node.skip {
        v.visit_expression(e);
    }
    if let Some(e) = &mut node.limit {
        v.visit_expression(e);
    }
}

pub fn walk_projection_item_mut<V: VisitMut>(v: &mut V, node: &mut ProjectionItem) {
    v.visit_expression(&mut node.expression);
    if let Some(var) = &mut node.alias {
        v.visit_variable(var);
    }
}

pub fn walk_order_mut<V: VisitMut>(v: &mut V, node: &mut Order) {
    for s in &mut node.items {
        v.visit_sort_item(s);
    }
}

pub fn walk_sort_item_mut<V: VisitMut>(v: &mut V, node: &mut SortItem) {
    v.visit_expression(&mut node.expression);
}

pub fn walk_unwind_mut<V: VisitMut>(v: &mut V, node: &mut Unwind) {
    v.visit_expression(&mut node.expression);
    v.visit_variable(&mut node.variable);
}

pub fn walk_standalone_call_mut<V: VisitMut>(v: &mut V, node: &mut StandaloneCall) {
    v.visit_procedure_invocation(&mut node.call);
    if let Some(y) = &mut node.yield_items {
        match y {
            YieldSpec::Star { .. } => {}
            YieldSpec::Items(yi) => v.visit_yield_items(yi),
        }
    }
}

pub fn walk_in_query_call_mut<V: VisitMut>(v: &mut V, node: &mut InQueryCall) {
    v.visit_procedure_invocation(&mut node.call);
    if let Some(y) = &mut node.yield_items {
        v.visit_yield_items(y);
    }
}

pub fn walk_procedure_invocation_mut<V: VisitMut>(v: &mut V, node: &mut ProcedureInvocation) {
    v.visit_function_invocation_expr(&mut node.name);
}

pub fn walk_yield_items_mut<V: VisitMut>(v: &mut V, node: &mut YieldItems) {
    for y in &mut node.items {
        v.visit_yield_item(y);
    }
    if let Some(e) = &mut node.where_clause {
        v.visit_expression(e);
    }
}

pub fn walk_yield_item_mut<V: VisitMut>(v: &mut V, node: &mut YieldItem) {
    v.visit_symbolic_name(&mut node.procedure_field);
    if let Some(var) = &mut node.alias {
        v.visit_variable(var);
    }
}

pub fn walk_pattern_mut<V: VisitMut>(v: &mut V, node: &mut Pattern) {
    for p in &mut node.parts {
        v.visit_pattern_part(p);
    }
}

pub fn walk_pattern_part_mut<V: VisitMut>(v: &mut V, node: &mut PatternPart) {
    if let Some(var) = &mut node.variable {
        v.visit_variable(var);
    }
    v.visit_anonymous_pattern_part(&mut node.anonymous);
}

pub fn walk_anonymous_pattern_part_mut<V: VisitMut>(v: &mut V, node: &mut AnonymousPatternPart) {
    v.visit_pattern_element(&mut node.element);
}

pub fn walk_pattern_element_mut<V: VisitMut>(v: &mut V, node: &mut PatternElement) {
    match node {
        PatternElement::Path { start, chains } => {
            v.visit_node_pattern(start);
            for c in chains {
                v.visit_pattern_element_chain(c);
            }
        }
        PatternElement::Parenthesized(inner) => {
            v.visit_pattern_element(inner);
        }
    }
}

pub fn walk_node_pattern_mut<V: VisitMut>(v: &mut V, node: &mut NodePattern) {
    if let Some(var) = &mut node.variable {
        v.visit_variable(var);
    }
    for l in &mut node.labels {
        v.visit_symbolic_name(l);
    }
    if let Some(props) = &mut node.properties {
        v.visit_properties(props);
    }
}

pub fn walk_pattern_element_chain_mut<V: VisitMut>(v: &mut V, node: &mut PatternElementChain) {
    v.visit_relationship_pattern(&mut node.relationship);
    v.visit_node_pattern(&mut node.node);
}

pub fn walk_relationship_pattern_mut<V: VisitMut>(v: &mut V, node: &mut RelationshipPattern) {
    if let Some(d) = &mut node.detail {
        v.visit_relationship_detail(d);
    }
}

pub fn walk_relationship_detail_mut<V: VisitMut>(v: &mut V, node: &mut RelationshipDetail) {
    if let Some(var) = &mut node.variable {
        v.visit_variable(var);
    }
    for t in &mut node.types {
        v.visit_rel_type_name(t);
    }
    if let Some(r) = &mut node.range {
        v.visit_range_literal(r);
    }
    if let Some(props) = &mut node.properties {
        v.visit_properties(props);
    }
}

pub fn walk_range_literal_mut<V: VisitMut>(_v: &mut V, _node: &mut RangeLiteral) {}

pub fn walk_relationships_pattern_mut<V: VisitMut>(v: &mut V, node: &mut RelationshipsPattern) {
    v.visit_node_pattern(&mut node.start);
    for c in &mut node.chains {
        v.visit_pattern_element_chain(c);
    }
}

pub fn walk_expression_mut<V: VisitMut>(v: &mut V, node: &mut Expression) {
    match node {
        Expression::Literal(l) => v.visit_literal(l),
        Expression::Variable(var) => v.visit_variable(var),
        Expression::Parameter(p) => v.visit_parameter(p),
        Expression::PropertyLookup { base, property, .. } => {
            v.visit_expression(base);
            v.visit_property_key_name(property);
        }
        Expression::NodeLabels { base, labels, .. } => {
            v.visit_expression(base);
            for l in labels {
                v.visit_symbolic_name(l);
            }
        }
        Expression::BinaryOp { lhs, rhs, .. } => {
            v.visit_expression(lhs);
            v.visit_expression(rhs);
        }
        Expression::UnaryOp { operand, .. } => {
            v.visit_expression(operand);
        }
        Expression::Comparison { lhs, operators, .. } => {
            v.visit_expression(lhs);
            for (_, rhs) in operators {
                v.visit_expression(rhs);
            }
        }
        Expression::ListIndex { list, index, .. } => {
            v.visit_expression(list);
            v.visit_expression(index);
        }
        Expression::ListSlice {
            list, start, end, ..
        } => {
            v.visit_expression(list);
            if let Some(s) = start {
                v.visit_expression(s);
            }
            if let Some(e) = end {
                v.visit_expression(e);
            }
        }
        Expression::In { lhs, rhs, .. } => {
            v.visit_expression(lhs);
            v.visit_expression(rhs);
        }
        Expression::IsNull { operand, .. } => {
            v.visit_expression(operand);
        }
        Expression::FunctionCall(func) => v.visit_function_invocation_expr(func),
        Expression::CountStar { .. } => {}
        Expression::Case(case) => v.visit_case_expression(case),
        Expression::ListComprehension(lc) => v.visit_list_comprehension(lc),
        Expression::PatternComprehension(pc) => v.visit_pattern_comprehension(pc),
        Expression::All(fe)
        | Expression::Any(fe)
        | Expression::None(fe)
        | Expression::Single(fe) => v.visit_filter_expression(fe),
        Expression::Parenthesized(inner) => v.visit_expression(inner),
        Expression::Pattern(rp) => v.visit_relationships_pattern(rp),
        Expression::Exists(exists) => v.visit_exists_expression(exists),
    }
}

pub fn walk_literal_mut<V: VisitMut>(v: &mut V, node: &mut Literal) {
    match node {
        Literal::Number(n) => v.visit_number_literal(n),
        Literal::String(s) => v.visit_string_literal(s),
        Literal::Boolean(_) => {}
        Literal::Null => {}
        Literal::List(l) => v.visit_list_literal(l),
        Literal::Map(m) => v.visit_map_literal(m),
    }
}

pub fn walk_number_literal_mut<V: VisitMut>(_v: &mut V, _node: &mut NumberLiteral) {}

pub fn walk_string_literal_mut<V: VisitMut>(_v: &mut V, _node: &mut StringLiteral) {}

pub fn walk_list_literal_mut<V: VisitMut>(v: &mut V, node: &mut ListLiteral) {
    for e in &mut node.elements {
        v.visit_expression(e);
    }
}

pub fn walk_map_literal_mut<V: VisitMut>(v: &mut V, node: &mut MapLiteral) {
    for (key, val) in &mut node.entries {
        v.visit_property_key_name(key);
        v.visit_expression(val);
    }
}

pub fn walk_parameter_mut<V: VisitMut>(v: &mut V, node: &mut Parameter) {
    v.visit_symbolic_name(&mut node.name);
}

pub fn walk_function_invocation_expr_mut<V: VisitMut>(v: &mut V, node: &mut FunctionInvocation) {
    for n in &mut node.name {
        v.visit_symbolic_name(n);
    }
    for a in &mut node.arguments {
        v.visit_expression(a);
    }
}

pub fn walk_case_expression_mut<V: VisitMut>(v: &mut V, node: &mut CaseExpression) {
    if let Some(s) = &mut node.scrutinee {
        v.visit_expression(s);
    }
    for a in &mut node.alternatives {
        v.visit_case_alternative(a);
    }
    if let Some(d) = &mut node.default {
        v.visit_expression(d);
    }
}

pub fn walk_case_alternative_mut<V: VisitMut>(v: &mut V, node: &mut CaseAlternative) {
    v.visit_expression(&mut node.when);
    v.visit_expression(&mut node.then);
}

pub fn walk_list_comprehension_mut<V: VisitMut>(v: &mut V, node: &mut ListComprehension) {
    v.visit_variable(&mut node.variable);
    if let Some(f) = &mut node.filter {
        v.visit_expression(f);
    }
    if let Some(m) = &mut node.map {
        v.visit_expression(m);
    }
}

pub fn walk_pattern_comprehension_mut<V: VisitMut>(v: &mut V, node: &mut PatternComprehension) {
    if let Some(var) = &mut node.variable {
        v.visit_variable(var);
    }
    v.visit_relationships_pattern(&mut node.pattern);
    if let Some(w) = &mut node.where_clause {
        v.visit_expression(w);
    }
    v.visit_expression(&mut node.map);
}

pub fn walk_filter_expression_mut<V: VisitMut>(v: &mut V, node: &mut FilterExpression) {
    v.visit_variable(&mut node.variable);
    v.visit_expression(&mut node.collection);
    if let Some(p) = &mut node.predicate {
        v.visit_expression(p);
    }
}

pub fn walk_exists_expression_mut<V: VisitMut>(v: &mut V, node: &mut ExistsExpression) {
    v.visit_exists_inner(&mut node.inner);
}

pub fn walk_exists_inner_mut<V: VisitMut>(v: &mut V, node: &mut ExistsInner) {
    match node {
        ExistsInner::Pattern(p, where_clause) => {
            v.visit_pattern(p);
            if let Some(w) = where_clause {
                v.visit_expression(w);
            }
        }
        ExistsInner::RegularQuery(rq) => v.visit_regular_query(rq),
    }
}

pub fn walk_variable_mut<V: VisitMut>(v: &mut V, node: &mut Variable) {
    v.visit_symbolic_name(&mut node.name);
}

pub fn walk_symbolic_name_mut<V: VisitMut>(_v: &mut V, _node: &mut SymbolicName) {}

pub fn walk_properties_mut<V: VisitMut>(v: &mut V, node: &mut Properties) {
    match node {
        Properties::Map(m) => v.visit_map_literal(m),
        Properties::Parameter(p) => v.visit_parameter(p),
    }
}

pub fn walk_rel_type_name_mut<V: VisitMut>(v: &mut V, node: &mut RelTypeName) {
    v.visit_symbolic_name(&mut node.name);
}

pub fn walk_property_key_name_mut<V: VisitMut>(v: &mut V, node: &mut PropertyKeyName) {
    v.visit_symbolic_name(&mut node.name);
}
