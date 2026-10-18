//! Name-resolution pass over a parsed Cypher query.
//!
//! This module implements the [`Visit`] trait to walk the AST and verify that
//! every variable reference can be resolved in the visible scope. Undeclared
//! variables produce [`SemaError::UnresolvedVariable`] diagnostics; duplicate
//! declarations produce [`SemaError::RedeclaredVariable`] diagnostics.
//!
//! The entry point is [`resolve_names`].

use crate::ast::ToCypher;
use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::pattern::*;
use crate::ast::query::*;
use crate::ast::visit::{
    Visit, walk_create, walk_expression, walk_match, walk_merge, walk_single_query,
};
use crate::error::CypherError;
use crate::sema::AggregateRegistry;
use crate::sema::error::SemaError;
use crate::sema::scope::{ScopeStack, SymbolKind};

/// The result of the name-resolution pass.
pub struct ResolutionResult {
    /// Errors collected during resolution.
    pub errors: Vec<CypherError>,
}

/// Run name resolution over `query`.
///
/// Returns `Ok(())` when every variable reference is bound, or
/// `Err(errors)` with all resolution violations found.
pub fn resolve_names(query: &Query) -> Result<(), Vec<CypherError>> {
    resolve_names_with(query, &AggregateRegistry::default())
}

/// Run name resolution over `query`, treating the functions in `aggregates`
/// as aggregates when deciding what a `WITH`'s `WHERE` and `ORDER BY` see.
///
/// Returns `Ok(())` when every variable reference is bound, or
/// `Err(errors)` with all resolution violations found.
pub fn resolve_names_with(
    query: &Query,
    aggregates: &AggregateRegistry,
) -> Result<(), Vec<CypherError>> {
    let mut resolver = NameResolver::new(aggregates.clone());
    resolver.visit_query(query);
    if resolver.errors.is_empty() {
        Ok(())
    } else {
        Err(resolver.errors)
    }
}

/// Visitor that resolves variable references and tracks scope.
struct NameResolver {
    /// Errors accumulated during the walk.
    errors: Vec<CypherError>,
    /// The current lexical scope stack.
    scopes: ScopeStack,
    /// The functions treated as aggregates.
    aggregates: AggregateRegistry,
    /// While visiting the `WHERE` or `ORDER BY` of a projection that
    /// aggregates or is `DISTINCT`: its printed item expressions, which stand
    /// for their columns there (`RETURN n.a, count(*) ORDER BY n.a`).
    projected: Vec<String>,
}

impl NameResolver {
    /// Create a fresh resolver with an empty scope stack.
    fn new(aggregates: AggregateRegistry) -> Self {
        Self {
            errors: Vec::new(),
            scopes: ScopeStack::new(),
            aggregates,
            projected: Vec::new(),
        }
    }

    /// `RETURN *` without further items needs a variable in scope. (`WITH *`
    /// may pass nothing on, as in `CREATE () WITH * CREATE ()`.)
    fn check_star_scope_of(
        &mut self,
        scopes: &ScopeStack,
        only_star: bool,
        span: crate::error::Span,
    ) {
        if only_star && scopes.visible_bindings().is_empty() {
            self.emit(SemaError::NoVariablesInScope { span });
        }
    }

    fn has_aggregate(&self, items: &[ProjectionItem]) -> bool {
        items.iter().any(|item| {
            item.expression
                .contains_aggregate(&|fc| self.aggregates.is_aggregate_call(fc))
        })
    }

    /// Runs `visit` on the `WHERE` or `ORDER BY` of a projection, with the
    /// columns of the projection in `self.scopes`.
    ///
    /// These also see the variables the projection read (`before`), as the
    /// openCypher TCK requires (`WITH a.name AS name WHERE a.age > 1`),
    /// unless the projection `narrow`s its rows: aggregation for both, and
    /// `DISTINCT` for `ORDER BY`. Then only the columns are visible, plus
    /// expressions repeating an item.
    fn visit_after_projection(
        &mut self,
        before: &ScopeStack,
        items: &[ProjectionItem],
        narrow: bool,
        visit: impl FnOnce(&mut Self),
    ) {
        let columns = self.scopes.clone();
        let outer_projected = std::mem::take(&mut self.projected);
        if narrow {
            self.projected = items
                .iter()
                .map(|item| item.expression.to_cypher())
                .collect();
        } else {
            let mut both = before.clone();
            both.push_scope();
            for (name, entry) in columns.visible_bindings() {
                // Fresh scope, so binding can't conflict.
                let _ = both.bind(&name, entry.kind, entry.span);
            }
            self.scopes = both;
        }
        visit(self);
        self.projected = outer_projected;
        self.scopes = columns;
    }

    /// Emit a semantic error, converting it to a [`CypherError`] first.
    /// Reports projection items that produce the same column name
    /// (`RETURN 1 AS a, 2 AS a`, `RETURN count(*), count(*)`, `WITH n, n`).
    fn report_duplicate_columns(&mut self, items: &[ProjectionItem]) {
        let mut seen: std::collections::HashMap<String, crate::error::Span> =
            std::collections::HashMap::new();
        for item in items {
            let (name, span) = item.column();
            if let Some(&first_span) = seen.get(&name) {
                self.emit(SemaError::RedeclaredVariable {
                    name,
                    first_span,
                    redecl_span: span,
                });
            } else {
                seen.insert(name, span);
            }
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
                SemaError::AggregateNotAllowed { span, .. } => *span,
                SemaError::NoVariablesInScope { span } => *span,
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
            false,
            &mut self.errors,
        );
        walk_match(self, node);
    }

    fn visit_create(&mut self, node: &'ast Create) {
        bind_pattern(
            &mut self.scopes,
            &node.pattern,
            SymbolKind::PatternBound,
            true,
            &mut self.errors,
        );
        walk_create(self, node);
    }

    fn visit_merge(&mut self, node: &'ast Merge) {
        bind_pattern_part(
            &mut self.scopes,
            &node.pattern,
            SymbolKind::PatternBound,
            true,
            &mut self.errors,
        );
        walk_merge(self, node);
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

        if node.star {
            projected.extend(
                self.scopes
                    .visible_bindings()
                    .into_iter()
                    .map(|(name, entry)| (name, entry.span)),
            );
        }

        self.report_duplicate_columns(&node.items);
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

        let before = std::mem::take(&mut self.scopes);

        for (bind_name, bind_span) in projected {
            // Duplicate columns were reported above.
            let _ = self
                .scopes
                .bind(&bind_name, SymbolKind::WithAlias, bind_span);
        }

        // SKIP and LIMIT are evaluated in the projected scope; WHERE and
        // ORDER BY may also see the variables the WITH read.
        let aggregating = self.has_aggregate(&node.items);
        if let Some(order) = &node.order {
            self.visit_after_projection(
                &before,
                &node.items,
                aggregating || node.distinct,
                |this| this.visit_order(order),
            );
        }
        if let Some(skip) = &node.skip {
            self.visit_expression(skip);
        }
        if let Some(limit) = &node.limit {
            self.visit_expression(limit);
        }
        if let Some(wc) = &node.where_clause {
            self.visit_after_projection(&before, &node.items, aggregating, |this| {
                this.visit_expression(wc)
            });
        }
    }

    fn visit_return(&mut self, node: &'ast Return) {
        self.report_duplicate_columns(&node.items);
        // All items are evaluated before any alias becomes visible: in
        // `RETURN a.id AS a, a IS NOT NULL AS b` the second `a` is still the
        // matched node.
        for item in &node.items {
            self.visit_expression(&item.expression);
        }
        // ORDER BY sees the columns; unless the RETURN aggregates or is
        // DISTINCT, also the variables it read, which aliases may shadow
        // (`AS a` above).
        let before = std::mem::take(&mut self.scopes);
        if node.star {
            self.check_star_scope_of(&before, node.items.is_empty(), node.span);
            for (name, entry) in before.visible_bindings() {
                let _ = self.scopes.bind(&name, entry.kind, entry.span);
            }
        }
        self.scopes.push_scope();
        for (name, span) in node.items.iter().filter_map(|item| match &item.alias {
            Some(alias) => Some((alias.name.name.clone(), alias.name.span)),
            None => derive_projection_name(&item.expression),
        }) {
            // Duplicate columns were reported above.
            let _ = self.scopes.bind(&name, SymbolKind::ReturnAlias, span);
        }
        if let Some(order) = &node.order {
            let narrow = node.distinct || self.has_aggregate(&node.items);
            self.visit_after_projection(&before, &node.items, narrow, |this| {
                this.visit_order(order)
            });
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
                // `YIELD field` binds `field`; `YIELD field AS alias` binds `alias`.
                let (name, span) = match &item.alias {
                    Some(alias) => (&alias.name.name, alias.name.span),
                    None => (&item.procedure_field.name, item.procedure_field.span),
                };
                if let Err(first_span) = self.scopes.bind(name, SymbolKind::YieldAlias, span) {
                    self.emit(SemaError::RedeclaredVariable {
                        name: name.clone(),
                        first_span,
                        redecl_span: span,
                    });
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
                        let (name, span) = match &item.alias {
                            Some(alias) => (&alias.name.name, alias.name.span),
                            None => (&item.procedure_field.name, item.procedure_field.span),
                        };
                        if let Err(first_span) =
                            self.scopes.bind(name, SymbolKind::YieldAlias, span)
                        {
                            self.emit(SemaError::RedeclaredVariable {
                                name: name.clone(),
                                first_span,
                                redecl_span: span,
                            });
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
        // The subquery sees the outer variables (its leading `WITH` imports
        // them and then replaces the scope), but nothing it binds leaks out
        // except the columns it returns.
        let outer = self.scopes.clone();
        self.visit_regular_query(&node.query);
        self.scopes = outer;
        for (name, span) in node.query.returned_columns() {
            if let Err(first_span) = self.scopes.bind(&name, SymbolKind::ReturnAlias, span) {
                self.emit(SemaError::RedeclaredVariable {
                    name,
                    first_span,
                    redecl_span: span,
                });
            }
        }
        if let Some(it) = &node.in_transactions
            && let Some(of_rows) = &it.of_rows
        {
            self.visit_expression(of_rows);
        }
    }

    fn visit_regular_query(&mut self, node: &'ast RegularQuery) {
        // A subquery body does not refer to the columns of an enclosing
        // projection by repeating its expressions.
        let outer_projected = std::mem::take(&mut self.projected);
        // Every UNION branch starts from the scope the query was entered with.
        let entry = self.scopes.clone();
        self.visit_single_query(&node.single_query);
        for union in &node.unions {
            self.scopes = entry.clone();
            self.visit_single_query(&union.single_query);
        }
        self.projected = outer_projected;
    }

    fn visit_exists_inner(&mut self, node: &'ast ExistsInner) {
        let outer = self.scopes.clone();
        match node {
            ExistsInner::Pattern(pattern, where_clause) => {
                // New pattern variables are local to the predicate.
                self.scopes.push_scope();
                bind_pattern(
                    &mut self.scopes,
                    pattern,
                    SymbolKind::PatternBound,
                    false,
                    &mut self.errors,
                );
                if let Some(w) = where_clause {
                    self.visit_expression(w);
                }
            }
            ExistsInner::RegularQuery(rq) => self.visit_regular_query(rq),
        }
        self.scopes = outer;
    }

    fn visit_count_subquery_expression(&mut self, node: &'ast CountSubqueryExpression) {
        self.visit_exists_inner(&node.inner);
    }

    fn visit_collect_subquery_expression(&mut self, node: &'ast CollectSubqueryExpression) {
        let outer = self.scopes.clone();
        self.visit_regular_query(&node.query);
        self.scopes = outer;
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
        // The collection is evaluated in the enclosing scope.
        self.visit_expression(&node.collection);
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
        if let Some(var) = &node.variable
            && let Err(first_span) =
                self.scopes
                    .bind(&var.name.name, SymbolKind::ComprehensionVar, var.name.span)
        {
            self.emit(SemaError::RedeclaredVariable {
                name: var.name.name.clone(),
                first_span,
                redecl_span: var.name.span,
            });
        }
        bind_relationships_pattern(&mut self.scopes, &node.pattern, SymbolKind::PatternBound);
        if let Some(wc) = &node.where_clause {
            self.visit_expression(wc);
        }
        self.visit_expression(&node.map);
        self.scopes.pop_scope();
    }

    fn visit_filter_expression(&mut self, node: &'ast FilterExpression) {
        // The collection is evaluated in the enclosing scope.
        self.visit_expression(&node.collection);
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
        if let Some(pred) = &node.predicate {
            self.visit_expression(pred);
        }
        self.scopes.pop_scope();
    }

    fn visit_reduce_expression(&mut self, node: &'ast ReduceExpression) {
        // The initial value and the list are evaluated in the enclosing scope;
        // the accumulator and element variable are visible only in the
        // reduction expression.
        self.visit_expression(&node.init);
        self.visit_expression(&node.list);
        self.scopes.push_scope();
        for var in [&node.accumulator, &node.variable] {
            if let Err(first_span) =
                self.scopes
                    .bind(&var.name.name, SymbolKind::ComprehensionVar, var.name.span)
            {
                self.emit(SemaError::RedeclaredVariable {
                    name: var.name.name.clone(),
                    first_span,
                    redecl_span: var.name.span,
                });
            }
        }
        self.visit_expression(&node.expression);
        self.scopes.pop_scope();
    }

    fn visit_expression(&mut self, node: &'ast Expression) {
        if self.projected.is_empty() || !self.projected.contains(&node.to_cypher()) {
            walk_expression(self, node);
        }
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

/// Bind all variables found in a pattern to the current scope. With
/// `creating` (`CREATE`, `MERGE`) the pattern must not re-create bound
/// entities.
fn bind_pattern(
    scopes: &mut ScopeStack,
    pattern: &Pattern,
    kind: SymbolKind,
    creating: bool,
    errors: &mut Vec<CypherError>,
) {
    for part in &pattern.parts {
        bind_pattern_part(scopes, part, kind, creating, errors);
    }
}

/// Binds a pattern part's node and relationship variables, then its path
/// variable (`p = …`), which must be new: neither bound before nor used for a
/// node or relationship of its own pattern (`MATCH p = (p)-->()`).
fn bind_pattern_part(
    scopes: &mut ScopeStack,
    part: &PatternPart,
    kind: SymbolKind,
    creating: bool,
    errors: &mut Vec<CypherError>,
) {
    bind_node_pattern(scopes, &part.anonymous.element, kind, creating, errors);
    if let Some(var) = &part.variable {
        let first_span = match scopes.resolve(&var.name.name) {
            Some((entry, _)) => Some(entry.span),
            None => scopes.bind(&var.name.name, kind, var.name.span).err(),
        };
        if let Some(first_span) = first_span {
            errors.push(redeclared(var, first_span));
        }
    }
}

fn redeclared(var: &crate::ast::names::Variable, first_span: crate::error::Span) -> CypherError {
    SemaError::RedeclaredVariable {
        name: var.name.name.clone(),
        first_span,
        redecl_span: var.name.span,
    }
    .into_error()
}

fn bind_node_pattern(
    scopes: &mut ScopeStack,
    element: &PatternElement,
    kind: SymbolKind,
    creating: bool,
    errors: &mut Vec<CypherError>,
) {
    match element {
        PatternElement::Path { start, chains } => {
            bind_node(scopes, start, chains.is_empty(), kind, creating, errors);
            for chain in chains {
                if let Some(var) = chain
                    .relationship
                    .detail
                    .as_ref()
                    .and_then(|d| d.variable.as_ref())
                {
                    // A created relationship is always new.
                    match scopes.resolve(&var.name.name) {
                        Some((entry, _)) if creating => errors.push(redeclared(var, entry.span)),
                        _ => bind_or_reuse(scopes, var, kind),
                    }
                }
                bind_node(scopes, &chain.node, false, kind, creating, errors);
            }
        }
        PatternElement::Parenthesized(inner) => {
            bind_node_pattern(scopes, inner, kind, creating, errors);
        }
        PatternElement::Quantified { element, .. } => {
            bind_node_pattern(scopes, element, kind, creating, errors);
        }
    }
}

/// Binds a node variable of a pattern. In `CREATE` / `MERGE` a bound node
/// can only be connected to: it may not stand alone (`MATCH (a) CREATE (a)`)
/// or get labels or properties (`MATCH (a) CREATE (a:Foo)-[:R]->()`).
fn bind_node(
    scopes: &mut ScopeStack,
    node: &NodePattern,
    standalone: bool,
    kind: SymbolKind,
    creating: bool,
    errors: &mut Vec<CypherError>,
) {
    let Some(var) = &node.variable else { return };
    if creating
        && (standalone || !node.labels.is_empty() || node.properties.is_some())
        && let Some((entry, _)) = scopes.resolve(&var.name.name)
    {
        errors.push(redeclared(var, entry.span));
        return;
    }
    bind_or_reuse(scopes, var, kind);
}

/// Binds a node or relationship variable of a pattern. A name that is
/// already visible refers to the same entity (`MATCH (n) MATCH (n)-->(m)`),
/// which is a reference rather than a redeclaration.
fn bind_or_reuse(scopes: &mut ScopeStack, var: &crate::ast::names::Variable, kind: SymbolKind) {
    if scopes.resolve(&var.name.name).is_none() {
        // Can't fail: the name isn't visible, so it isn't in the current scope.
        let _ = scopes.bind(&var.name.name, kind, var.name.span);
    }
}

fn bind_relationships_pattern(
    scopes: &mut ScopeStack,
    pattern: &RelationshipsPattern,
    kind: SymbolKind,
) {
    if let Some(var) = &pattern.start.variable {
        bind_or_reuse(scopes, var, kind);
    }
    for chain in &pattern.chains {
        if let Some(var) = chain
            .relationship
            .detail
            .as_ref()
            .and_then(|d| d.variable.as_ref())
        {
            bind_or_reuse(scopes, var, kind);
        }
        if let Some(var) = &chain.node.variable {
            bind_or_reuse(scopes, var, kind);
        }
    }
}

/// Derive the projected name from an unaliased expression.
/// Returns (name, span) only for plain variable projections.
///
/// Cypher requires `AS` to introduce a new variable for property lookups
/// and other expressions.
fn derive_projection_name(expr: &Expression) -> Option<(String, crate::error::Span)> {
    match expr {
        Expression::Variable(v) => Some((v.name.name.clone(), v.name.span)),
        _ => None, // Property lookups, literals, function calls, etc. need `AS`
    }
}
