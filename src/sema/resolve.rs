//! Name-resolution pass over a Query.

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::pattern::*;
use crate::ast::query::*;
use crate::ast::visit::{Visit, walk_match, walk_single_query};
use crate::error::CypherError;
use crate::sema::error::SemaError;
use crate::sema::scope::{ScopeStack, SymbolKind};

pub struct ResolutionResult {
    pub errors: Vec<CypherError>,
}

/// Run name resolution over a query, returning any errors found.
pub fn resolve_names(query: &Query) -> Result<(), Vec<CypherError>> {
    let mut resolver = NameResolver::new();
    resolver.visit_query(query);
    if resolver.errors.is_empty() {
        Ok(())
    } else {
        Err(resolver.errors)
    }
}

struct NameResolver {
    errors: Vec<CypherError>,
    scopes: ScopeStack,
}

impl NameResolver {
    fn new() -> Self {
        Self {
            errors: Vec::new(),
            scopes: ScopeStack::new(),
        }
    }

    fn emit(&mut self, sema: SemaError) {
        self.errors.push(CypherError {
            kind: sema.to_error_kind(),
            span: match &sema {
                SemaError::UnresolvedVariable { span, .. } => *span,
                SemaError::RedeclaredVariable { redecl_span, .. } => *redecl_span,
                SemaError::AggregationMix { span, .. } => *span,
                SemaError::DistinctNotAllowed { span } => *span,
                SemaError::InvalidReference { span, .. } => *span,
            },
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    }
}

impl<'ast> Visit<'ast> for NameResolver {
    fn visit_single_query(&mut self, node: &'ast SingleQuery) {
        // Override to handle WITH scope boundaries in multi-part queries
        match &node.kind {
            SingleQueryKind::MultiPart(mp) => {
                for part in &mp.parts {
                    // Visit reading clauses
                    for rc in &part.reading_clauses {
                        match rc {
                            ReadingClause::Match(m) => self.visit_match(m),
                            ReadingClause::Unwind(u) => self.visit_unwind(u),
                            ReadingClause::InQueryCall(i) => self.visit_in_query_call(i),
                            ReadingClause::CallSubquery(c) => self.visit_call_subquery(c),
                            ReadingClause::LoadCsv(l) => self.visit_load_csv(l),
                        }
                    }
                    // Visit updating clauses
                    for uc in &part.updating_clauses {
                        match uc {
                            UpdatingClause::Create(c) => self.visit_create(c),
                            UpdatingClause::Merge(m) => self.visit_merge(m),
                            UpdatingClause::Delete(d) => self.visit_delete(d),
                            UpdatingClause::Set(s) => self.visit_set(s),
                            UpdatingClause::Remove(r) => self.visit_remove(r),
                            UpdatingClause::Foreach(f) => self.visit_foreach(f),
                        }
                    }
                    self.visit_with(&part.with);
                }
                // Visit final part (reading clauses + body)
                for rc in &mp.final_part.reading_clauses {
                    match rc {
                        ReadingClause::Match(m) => self.visit_match(m),
                        ReadingClause::Unwind(u) => self.visit_unwind(u),
                        ReadingClause::InQueryCall(i) => self.visit_in_query_call(i),
                        ReadingClause::CallSubquery(c) => self.visit_call_subquery(c),
                        ReadingClause::LoadCsv(l) => self.visit_load_csv(l),
                    }
                }
                match &mp.final_part.body {
                    SinglePartBody::Return(r) => self.visit_return(r),
                    SinglePartBody::Updating {
                        updating,
                        return_clause,
                    } => {
                        for uc in updating {
                            match uc {
                                UpdatingClause::Create(c) => self.visit_create(c),
                                UpdatingClause::Merge(m) => self.visit_merge(m),
                                UpdatingClause::Delete(d) => self.visit_delete(d),
                                UpdatingClause::Set(s) => self.visit_set(s),
                                UpdatingClause::Remove(r) => self.visit_remove(r),
                                UpdatingClause::Foreach(f) => self.visit_foreach(f),
                            }
                        }
                        if let Some(ret) = return_clause {
                            self.visit_return(ret);
                        }
                    }
                    SinglePartBody::Finish(_) => {}
                }
            }
            SingleQueryKind::SinglePart(_) => {
                // Single-part queries don't have WITH boundaries — delegate to default walker
                walk_single_query(self, node);
            }
        }
    }

    fn visit_match(&mut self, node: &'ast Match) {
        // Bind pattern variables
        bind_pattern(
            &mut self.scopes,
            &node.pattern,
            SymbolKind::PatternBound,
            &mut self.errors,
        );
        walk_match(self, node);
    }

    fn visit_unwind(&mut self, node: &'ast Unwind) {
        // Unwind expression is evaluated first, then variable is bound
        self.visit_expression(&node.expression);
        if let Err(first_span) = self.scopes.bind(
            &node.variable.name.name,
            SymbolKind::UnwindBound,
            node.variable.name.span,
        ) {
            self.emit(SemaError::RedeclaredVariable {
                name: node.variable.name.name.clone(),
                first_span,
                redecl_span: node.variable.name.span,
            });
        }
    }

    fn visit_with(&mut self, node: &'ast With) {
        // WITH projection items are evaluated in the current scope.
        // Once projection completes, the visible query scope is replaced by
        // only the projected bindings.
        let mut projected = Vec::new();

        for item in &node.items {
            self.visit_expression(&item.expression);

            let bind = if let Some(alias) = &item.alias {
                Some((alias.name.name.clone(), alias.name.span))
            } else {
                derive_projection_name(&item.expression)
            };

            if let Some(binding) = bind {
                projected.push(binding);
            }
        }

        self.scopes = ScopeStack::new();

        for (bind_name, bind_span) in projected {
            if let Err(first_span) =
                self.scopes
                    .bind(&bind_name, SymbolKind::WithAlias, bind_span)
            {
                self.emit(SemaError::RedeclaredVariable {
                    name: bind_name,
                    first_span,
                    redecl_span: bind_span,
                });
            }
        }

        // ORDER BY, SKIP, LIMIT, WHERE are evaluated in the projected scope.
        if let Some(order) = &node.order {
            self.visit_order(order);
        }
        if let Some(skip) = &node.skip {
            self.visit_expression(skip);
        }
        if let Some(limit) = &node.limit {
            self.visit_expression(limit);
        }
        if let Some(wc) = &node.where_clause {
            self.visit_expression(wc);
        }
    }

    fn visit_return(&mut self, node: &'ast Return) {
        for item in &node.items {
            self.visit_expression(&item.expression);
            if let Some(alias) = &item.alias {
                if let Err(first_span) =
                    self.scopes
                        .bind(&alias.name.name, SymbolKind::ReturnAlias, alias.name.span)
                {
                    self.emit(SemaError::RedeclaredVariable {
                        name: alias.name.name.clone(),
                        first_span,
                        redecl_span: alias.name.span,
                    });
                }
            }
        }
        if let Some(order) = &node.order {
            self.visit_order(order);
        }
        if let Some(skip) = &node.skip {
            self.visit_expression(skip);
        }
        if let Some(limit) = &node.limit {
            self.visit_expression(limit);
        }
    }

    fn visit_in_query_call(&mut self, node: &'ast crate::ast::procedure::InQueryCall) {
        self.visit_procedure_invocation(&node.call);
        if let Some(yield_items) = &node.yield_items {
            for item in &yield_items.items {
                self.visit_symbolic_name(&item.procedure_field);
                if let Some(alias) = &item.alias {
                    if let Err(first_span) =
                        self.scopes
                            .bind(&alias.name.name, SymbolKind::YieldAlias, alias.name.span)
                    {
                        self.emit(SemaError::RedeclaredVariable {
                            name: alias.name.name.clone(),
                            first_span,
                            redecl_span: alias.name.span,
                        });
                    }
                }
            }
            if let Some(wc) = &yield_items.where_clause {
                self.visit_expression(wc);
            }
        }
    }

    fn visit_standalone_call(&mut self, node: &'ast crate::ast::procedure::StandaloneCall) {
        self.visit_procedure_invocation(&node.call);
        if let Some(yield_spec) = &node.yield_items {
            match yield_spec {
                crate::ast::procedure::YieldSpec::Star { .. } => {}
                crate::ast::procedure::YieldSpec::Items(yi) => {
                    for item in &yi.items {
                        self.visit_symbolic_name(&item.procedure_field);
                        if let Some(alias) = &item.alias {
                            if let Err(first_span) = self.scopes.bind(
                                &alias.name.name,
                                SymbolKind::YieldAlias,
                                alias.name.span,
                            ) {
                                self.emit(SemaError::RedeclaredVariable {
                                    name: alias.name.name.clone(),
                                    first_span,
                                    redecl_span: alias.name.span,
                                });
                            }
                        }
                    }
                    if let Some(wc) = &yi.where_clause {
                        self.visit_expression(wc);
                    }
                }
            }
        }
    }

    fn visit_call_subquery(&mut self, node: &'ast CallSubquery) {
        // Subqueries have their own scope — visit the inner query with a fresh scope
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.visit_regular_query(&node.query);
        self.scopes = saved_scopes;
        if let Some(it) = &node.in_transactions {
            if let Some(of_rows) = &it.of_rows {
                self.visit_expression(of_rows);
            }
        }
    }

    fn visit_foreach(&mut self, node: &'ast Foreach) {
        // FOREACH inner updates are scoped; the list expr is evaluated in outer scope
        self.visit_expression(&node.list);
        self.scopes.push_scope();
        if let Err(first_span) = self.scopes.bind(
            &node.variable.name.name,
            SymbolKind::ForeachVar,
            node.variable.name.span,
        ) {
            self.emit(SemaError::RedeclaredVariable {
                name: node.variable.name.name.clone(),
                first_span,
                redecl_span: node.variable.name.span,
            });
        }
        for update in &node.updates {
            self.visit_foreach_update(update);
        }
        self.scopes.pop_scope();
    }

    fn visit_list_comprehension(&mut self, node: &'ast ListComprehension) {
        self.scopes.push_scope();
        if let Err(first_span) = self.scopes.bind(
            &node.variable.name.name,
            SymbolKind::ComprehensionVar,
            node.variable.name.span,
        ) {
            self.emit(SemaError::RedeclaredVariable {
                name: node.variable.name.name.clone(),
                first_span,
                redecl_span: node.variable.name.span,
            });
        }
        if let Some(filter) = &node.filter {
            self.visit_expression(filter);
        }
        if let Some(map) = &node.map {
            self.visit_expression(map);
        }
        self.scopes.pop_scope();
    }

    fn visit_pattern_comprehension(&mut self, node: &'ast PatternComprehension) {
        self.scopes.push_scope();
        if let Some(var) = &node.variable {
            if let Err(first_span) = self
                .scopes
                .bind(&var.name.name, SymbolKind::ComprehensionVar, var.name.span)
            {
                self.emit(SemaError::RedeclaredVariable {
                    name: var.name.name.clone(),
                    first_span,
                    redecl_span: var.name.span,
                });
            }
        }
        bind_relationships_pattern(
            &mut self.scopes,
            &node.pattern,
            SymbolKind::PatternBound,
            &mut self.errors,
        );
        if let Some(wc) = &node.where_clause {
            self.visit_expression(wc);
        }
        self.visit_expression(&node.map);
        self.scopes.pop_scope();
    }

    fn visit_filter_expression(&mut self, node: &'ast FilterExpression) {
        self.scopes.push_scope();
        if let Err(first_span) = self.scopes.bind(
            &node.variable.name.name,
            SymbolKind::ComprehensionVar,
            node.variable.name.span,
        ) {
            self.emit(SemaError::RedeclaredVariable {
                name: node.variable.name.name.clone(),
                first_span,
                redecl_span: node.variable.name.span,
            });
        }
        self.visit_expression(&node.collection);
        if let Some(pred) = &node.predicate {
            self.visit_expression(pred);
        }
        self.scopes.pop_scope();
    }

    fn visit_variable(&mut self, node: &'ast crate::ast::names::Variable) {
        if !self.scopes.is_bound(&node.name.name) {
            self.emit(SemaError::UnresolvedVariable {
                name: node.name.name.clone(),
                span: node.name.span,
            });
        }
    }
}

/// Bind all variables found in a pattern to the current scope.
fn bind_pattern(
    scopes: &mut ScopeStack,
    pattern: &Pattern,
    kind: SymbolKind,
    errors: &mut Vec<CypherError>,
) {
    for part in &pattern.parts {
        if let Some(var) = &part.variable {
            if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
                errors.push(
                    SemaError::RedeclaredVariable {
                        name: var.name.name.clone(),
                        first_span,
                        redecl_span: var.name.span,
                    }
                    .into_error(),
                );
            }
        }
        bind_node_pattern(scopes, &part.anonymous.element, kind, errors);
    }
}

fn bind_node_pattern(
    scopes: &mut ScopeStack,
    element: &PatternElement,
    kind: SymbolKind,
    errors: &mut Vec<CypherError>,
) {
    match element {
        PatternElement::Path { start, chains } => {
            if let Some(var) = &start.variable {
                if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
                    errors.push(
                        SemaError::RedeclaredVariable {
                            name: var.name.name.clone(),
                            first_span,
                            redecl_span: var.name.span,
                        }
                        .into_error(),
                    );
                }
            }
            for chain in chains {
                if let Some(var) = &chain
                    .relationship
                    .detail
                    .as_ref()
                    .and_then(|d| d.variable.as_ref())
                {
                    if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
                        errors.push(
                            SemaError::RedeclaredVariable {
                                name: var.name.name.clone(),
                                first_span,
                                redecl_span: var.name.span,
                            }
                            .into_error(),
                        );
                    }
                }
                if let Some(var) = &chain.node.variable {
                    if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
                        errors.push(
                            SemaError::RedeclaredVariable {
                                name: var.name.name.clone(),
                                first_span,
                                redecl_span: var.name.span,
                            }
                            .into_error(),
                        );
                    }
                }
            }
        }
        PatternElement::Parenthesized(inner) => {
            bind_node_pattern(scopes, inner, kind, errors);
        }
    }
}

fn bind_relationships_pattern(
    scopes: &mut ScopeStack,
    pattern: &RelationshipsPattern,
    kind: SymbolKind,
    errors: &mut Vec<CypherError>,
) {
    if let Some(var) = &pattern.start.variable {
        if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
            errors.push(
                SemaError::RedeclaredVariable {
                    name: var.name.name.clone(),
                    first_span,
                    redecl_span: var.name.span,
                }
                .into_error(),
            );
        }
    }
    for chain in &pattern.chains {
        if let Some(var) = &chain
            .relationship
            .detail
            .as_ref()
            .and_then(|d| d.variable.as_ref())
        {
            if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
                errors.push(
                    SemaError::RedeclaredVariable {
                        name: var.name.name.clone(),
                        first_span,
                        redecl_span: var.name.span,
                    }
                    .into_error(),
                );
            }
        }
        if let Some(var) = &chain.node.variable {
            if let Err(first_span) = scopes.bind(&var.name.name, kind, var.name.span) {
                errors.push(
                    SemaError::RedeclaredVariable {
                        name: var.name.name.clone(),
                        first_span,
                        redecl_span: var.name.span,
                    }
                    .into_error(),
                );
            }
        }
    }
}

/// Derive the projected name from an unaliased expression.
/// Returns (name, span) for variable or property lookups, None for other expressions.
fn derive_projection_name(expr: &Expression) -> Option<(String, crate::error::Span)> {
    match expr {
        Expression::Variable(v) => Some((v.name.name.clone(), v.name.span)),
        Expression::PropertyLookup { property, .. } => {
            // For `n.name`, bind as `name`
            Some((property.name.name.clone(), property.name.span))
        }
        _ => None, // Literals, function calls, etc. — not bindable
    }
}
