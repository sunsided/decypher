//! AST → HIR lowering pass.
//!
//! This module is internal to the `hir` subsystem. It walks the parsed
//! [`crate::ast::query::Query`] AST top-down, resolves variable references
//! into arena [`BindingId`]s, normalises graph patterns into flat
//! [`GraphPattern`] lists, and emits a sequence of [`Operation`]s grouped
//! into [`QueryPart`]s.
//!
//! The public entry point is [`lower`].

use std::collections::HashMap;

use crate::ast::clause::{
    Create, Delete, Foreach, ForeachUpdate, LoadCsv, Match, Merge, Remove, RemoveItem, Return, Set,
    SetItem, SetOperator, Unwind, With,
};
use crate::ast::expr::{
    BinaryOperator, ComparisonOperator, ExistsInner, Expression, FunctionInvocation, Literal,
    MapProjectionItem, NumberLiteral, UnaryOperator,
};
use crate::ast::names::Variable;
use crate::ast::pattern::{
    LabelExpression, NodePattern, Pattern, PatternElement, Properties, RelationshipDirection,
    RelationshipPattern,
};
use crate::ast::procedure::{InQueryCall, ProcedureInvocation, YieldSpec};
use crate::ast::query::{
    CallSubquery, MultiPartQuery, OnErrorBehavior, Query, QueryBody, ReadingClause, RegularQuery,
    SinglePartBody, SinglePartQuery, SingleQuery, SingleQueryKind, UpdatingClause,
};
use crate::error::{Diagnostics, Span};

use super::arena::{BindingId, ExprId, FunctionId, Id, LabelId, PropertyKeyId, RelTypeId, ScopeId};
use super::binding::{Binding, BindingKind, Scope};
use super::config::LowerConfig;
use super::expr::{
    BinaryOp, CaseAlternative as HirCaseAlternative, CaseExpr as HirCaseExpr, CollectSubquery,
    CollectionQuantifier, ComparisonOperator as HirComparisonOperator, CountSubquery,
    ExistsSubquery, ExprKind, HirExpr, ListComprehension as HirListComprehension,
    Literal as HirLiteral, MapProjectionItem as HirMapProjectionItem,
    PatternComprehension as HirPatternComprehension, UnaryOp,
};
use super::ops::{
    AggregateItem, AggregateOp, CallProcedureOp, CallSubqueryOp, CreateOp, DeleteOp, FilterOp,
    ForeachOp, InTransactions as HirInTransactions, LimitOp, LoadCsvOp, MatchOp, MergeOp,
    OnErrorBehavior as HirOnErrorBehavior, Operation, ProcedureYield, ProjectOp, ProjectionItem,
    RemoveItem as HirRemoveItem, RemoveOp, ReturnOp, SetItem as HirSetItem, SetOp, SkipOp,
    SortDirection, SortItem, SortOp, UnwindOp,
};
use super::pattern::{
    GraphPattern, LabelExpr as HirLabelExpr, NodeIndex, NodePattern as HirNodePattern, PathBinding,
    RelIndex, RelationshipDirection as HirRelationshipDirection, RelationshipLength,
    RelationshipPattern as HirRelationshipPattern,
};
use super::{HirArenas, HirDiagnostic, HirQuery, QueryPart};

/// Lower an AST [`Query`] into a scope-resolved, normalised [`HirQuery`].
///
/// Returns `Ok(HirQuery)` when there are no fatal semantic errors; otherwise
/// returns `Err(Diagnostics)` containing all errors collected during lowering.
///
/// # Errors
///
/// Returns [`crate::error::Diagnostics`] on scope resolution or pattern
/// normalisation failures.
pub fn lower(query: &Query, config: &LowerConfig) -> Result<HirQuery, Diagnostics> {
    let mut ctx = LoweringContext::new(config);
    let mut parts = Vec::new();

    for stmt in &query.statements {
        let stmt_parts = ctx.lower_query_body(stmt);
        parts.extend(stmt_parts);
    }

    let arenas = ctx.arenas;
    let diagnostics = ctx.diagnostics;

    if diagnostics.is_empty() {
        Ok(HirQuery {
            arenas,
            parts,
            diagnostics: Vec::new(),
        })
    } else {
        Err(Diagnostics {
            errors: diagnostics
                .into_iter()
                .map(|d| d.into_error(None))
                .collect(),
        })
    }
}

/// Mutable lowering context threaded through the entire lowering pass.
///
/// Holds the growing [`HirArenas`], the list of emitted diagnostics, the
/// scope stack, and the buffer of operations being built for the current
/// query part.
struct LoweringContext<'cfg> {
    arenas: HirArenas,
    diagnostics: Vec<HirDiagnostic>,
    scope_stack: LoweringScopeStack,
    current_part_ops: Vec<Operation>,
    config: &'cfg LowerConfig,
}

/// Tracks variable bindings in a stack of scope frames during lowering.
struct LoweringScopeStack {
    /// The arena IDs of the currently active scopes, innermost last.
    scopes: Vec<ScopeId>,
    /// Per-scope name→BindingId maps, parallel to `scopes`.
    bindings_by_name: Vec<HashMap<String, BindingId>>,
}

impl LoweringScopeStack {
    /// Create an empty scope stack.
    fn new() -> Self {
        Self {
            scopes: Vec::new(),
            bindings_by_name: Vec::new(),
        }
    }

    /// Allocate a new scope in the arena and push it onto the stack.
    ///
    /// Returns the ID of the newly created scope.
    fn push_scope(&mut self, arenas: &mut HirArenas) -> ScopeId {
        let parent = self.scopes.last().copied();
        let scope = Scope {
            parent,
            bindings: Vec::new(),
        };
        let scope_id = arenas.scopes.alloc(scope);
        self.scopes.push(scope_id);
        self.bindings_by_name.push(HashMap::new());
        scope_id
    }

    /// Pop the innermost scope frame without deallocating its arena entry.
    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.bindings_by_name.pop();
    }

    /// Bind `name` in the current innermost scope.
    ///
    /// Allocates a new [`Binding`] in the arena, registers it in the scope,
    /// and returns its [`BindingId`]. If the same name is already present in
    /// the current scope the existing entry is silently overwritten. Callers
    /// that need to detect duplicate bindings (e.g. the semantic-analysis
    /// pass) must check for conflicts before calling this method.
    fn bind(
        &mut self,
        arenas: &mut HirArenas,
        name: &str,
        kind: BindingKind,
        span: Span,
    ) -> BindingId {
        let id = arenas.bindings.alloc(Binding {
            id: Id(usize::MAX), // temporary, updated below
            name: name.to_string(),
            kind,
            introduced_at: span,
        });
        // Update id inside the binding itself
        arenas.bindings.get_mut(id).id = id;

        let scope_id = *self.scopes.last().unwrap();
        arenas.scopes.get_mut(scope_id).bindings.push(id);

        let map = self.bindings_by_name.last_mut().unwrap();
        if let Some(&existing) = map.get(name) {
            let _first_span = arenas.bindings.get(existing).introduced_at;
            arenas.bindings.get_mut(id).id = id;
        }
        map.insert(name.to_string(), id);
        id
    }

    /// Resolve `name` by searching from the innermost to the outermost scope.
    ///
    /// Returns `None` if the name is not bound in any visible scope.
    fn resolve(&self, name: &str) -> Option<BindingId> {
        for map in self.bindings_by_name.iter().rev() {
            if let Some(&id) = map.get(name) {
                return Some(id);
            }
        }
        None
    }
}

impl<'cfg> LoweringContext<'cfg> {
    /// Create a fresh lowering context with empty arenas and scope stack.
    fn new(config: &'cfg LowerConfig) -> Self {
        Self {
            arenas: HirArenas::new(),
            diagnostics: Vec::new(),
            scope_stack: LoweringScopeStack::new(),
            current_part_ops: Vec::new(),
            config,
        }
    }

    /// Lower a single [`QueryBody`] statement into zero or more [`QueryPart`]s.
    fn lower_query_body(&mut self, stmt: &QueryBody) -> Vec<QueryPart> {
        let mut parts = Vec::new();
        match stmt {
            QueryBody::SingleQuery(sq) => {
                parts.extend(self.lower_single_query(sq));
            }
            QueryBody::Regular(rq) => {
                parts.extend(self.lower_regular_query(rq));
            }
            QueryBody::Standalone(call) => {
                let scope = self.scope_stack.push_scope(&mut self.arenas);
                let op = self.lower_standalone_call(call);
                self.current_part_ops.push(op);
                let out_scope = self.scope_stack.push_scope(&mut self.arenas);
                parts.push(QueryPart {
                    input_scope: scope,
                    operations: std::mem::take(&mut self.current_part_ops),
                    output_scope: out_scope,
                });
                self.scope_stack.pop_scope();
                self.scope_stack.pop_scope();
            }
            QueryBody::SchemaCommand(_) | QueryBody::Show(_) | QueryBody::Use(_) => {
                // Unsupported for now
            }
        }
        parts
    }

    fn lower_regular_query(&mut self, rq: &RegularQuery) -> Vec<QueryPart> {
        let mut parts = self.lower_single_query(&rq.single_query);
        for union in &rq.unions {
            let union_parts = self.lower_single_query(&union.single_query);
            // TODO: proper union handling
            parts.extend(union_parts);
        }
        parts
    }

    fn lower_single_query(&mut self, sq: &SingleQuery) -> Vec<QueryPart> {
        match &sq.kind {
            SingleQueryKind::SinglePart(sp) => self.lower_single_part_query(sp),
            SingleQueryKind::MultiPart(mp) => self.lower_multi_part_query(mp),
        }
    }

    fn lower_single_part_query(&mut self, sp: &SinglePartQuery) -> Vec<QueryPart> {
        let in_scope = self.scope_stack.push_scope(&mut self.arenas);

        for rc in &sp.reading_clauses {
            self.lower_reading_clause(rc);
        }

        match &sp.body {
            SinglePartBody::Return(ret) => {
                self.lower_return(ret);
            }
            SinglePartBody::Updating {
                updating,
                return_clause,
            } => {
                for uc in updating {
                    self.lower_updating_clause(uc);
                }
                if let Some(ret) = return_clause {
                    self.lower_return(ret);
                } else {
                    self.current_part_ops.push(Operation::Finish);
                }
            }
            SinglePartBody::Finish(_) => {
                self.current_part_ops.push(Operation::Finish);
            }
        }

        let out_scope = self.scope_stack.push_scope(&mut self.arenas);
        let part = QueryPart {
            input_scope: in_scope,
            operations: std::mem::take(&mut self.current_part_ops),
            output_scope: out_scope,
        };
        self.scope_stack.pop_scope();
        self.scope_stack.pop_scope();
        vec![part]
    }

    fn lower_multi_part_query(&mut self, mp: &MultiPartQuery) -> Vec<QueryPart> {
        let mut parts = Vec::new();
        let mut in_scope = self.scope_stack.push_scope(&mut self.arenas);

        for part in &mp.parts {
            for rc in &part.reading_clauses {
                self.lower_reading_clause(rc);
            }
            for uc in &part.updating_clauses {
                self.lower_updating_clause(uc);
            }
            self.lower_with(&part.with);

            let out_scope = self.scope_stack.push_scope(&mut self.arenas);
            parts.push(QueryPart {
                input_scope: in_scope,
                operations: std::mem::take(&mut self.current_part_ops),
                output_scope: out_scope,
            });
            self.scope_stack.pop_scope();
            // New scope for next part - WITH already pushed bindings
            in_scope = out_scope;
        }

        // Final part
        for rc in &mp.final_part.reading_clauses {
            self.lower_reading_clause(rc);
        }

        match &mp.final_part.body {
            SinglePartBody::Return(ret) => {
                self.lower_return(ret);
            }
            SinglePartBody::Updating {
                updating,
                return_clause,
            } => {
                for uc in updating {
                    self.lower_updating_clause(uc);
                }
                if let Some(ret) = return_clause {
                    self.lower_return(ret);
                } else {
                    self.current_part_ops.push(Operation::Finish);
                }
            }
            SinglePartBody::Finish(_) => {
                self.current_part_ops.push(Operation::Finish);
            }
        }

        let out_scope = self.scope_stack.push_scope(&mut self.arenas);
        parts.push(QueryPart {
            input_scope: in_scope,
            operations: std::mem::take(&mut self.current_part_ops),
            output_scope: out_scope,
        });
        self.scope_stack.pop_scope();
        self.scope_stack.pop_scope();

        parts
    }

    fn lower_reading_clause(&mut self, rc: &ReadingClause) {
        match rc {
            ReadingClause::Match(m) => self.lower_match(m),
            ReadingClause::Unwind(u) => self.lower_unwind(u),
            ReadingClause::InQueryCall(i) => self.lower_in_query_call(i),
            ReadingClause::CallSubquery(c) => self.lower_call_subquery(c),
            ReadingClause::LoadCsv(l) => self.lower_load_csv(l),
        }
    }

    fn lower_updating_clause(&mut self, uc: &UpdatingClause) {
        match uc {
            UpdatingClause::Create(c) => self.lower_create(c),
            UpdatingClause::Merge(m) => self.lower_merge(m),
            UpdatingClause::Delete(d) => self.lower_delete(d),
            UpdatingClause::Set(s) => self.lower_set(s),
            UpdatingClause::Remove(r) => self.lower_remove(r),
            UpdatingClause::Foreach(f) => self.lower_foreach(f),
        }
    }

    fn lower_match(&mut self, m: &Match) {
        let pattern = self.lower_pattern(&m.pattern);
        let mut predicates = Vec::new();
        if let Some(where_expr) = &m.where_clause {
            predicates.push(self.lower_expr(where_expr));
        }
        let op = if m.optional {
            Operation::OptionalMatch(MatchOp {
                pattern,
                predicates,
            })
        } else {
            Operation::Match(MatchOp {
                pattern,
                predicates,
            })
        };
        self.current_part_ops.push(op);
    }

    fn lower_unwind(&mut self, u: &Unwind) {
        let expr = self.lower_expr(&u.expression);
        let var = self.scope_stack.bind(
            &mut self.arenas,
            &u.variable.name.name,
            BindingKind::UnwindBound,
            u.variable.name.span,
        );
        self.current_part_ops.push(Operation::Unwind(UnwindOp {
            expression: expr,
            variable: var,
        }));
    }

    fn lower_in_query_call(&mut self, i: &InQueryCall) {
        let mut proc_op = self.lower_procedure_invocation(&i.call);
        let mut predicate = None;
        if let Some(yield_items) = &i.yield_items {
            for item in &yield_items.items {
                // `YIELD field` binds `field`; `YIELD field AS alias` binds `alias`.
                let (name, span) = match &item.alias {
                    Some(alias) => (&alias.name.name, alias.name.span),
                    None => (&item.procedure_field.name, item.procedure_field.span),
                };
                let alias =
                    self.scope_stack
                        .bind(&mut self.arenas, name, BindingKind::YieldAlias, span);
                proc_op.yields.push(ProcedureYield {
                    field: item.procedure_field.name.clone(),
                    alias,
                });
            }
            predicate = yield_items
                .where_clause
                .as_ref()
                .map(|w| self.lower_expr(w));
        }
        self.current_part_ops
            .push(Operation::CallProcedure(proc_op));
        if let Some(predicate) = predicate {
            self.current_part_ops
                .push(Operation::Filter(FilterOp { predicate }));
        }
    }

    /// Lowers the body of a `CALL`, `EXISTS`, `COUNT` or `COLLECT` subquery.
    ///
    /// The subquery gets its own arenas, but the outer variables visible at
    /// the call site are seeded into its root scope, so correlated references
    /// such as `CALL { WITH x … }` resolve. Diagnostics from the subquery are
    /// reported on the enclosing query instead of being dropped. Returns the
    /// lowered subquery and the outer bindings visible to it.
    fn lower_subquery(&mut self, query: &RegularQuery) -> (HirQuery, Vec<BindingId>) {
        // Innermost binding wins for shadowed names.
        let mut visible: HashMap<String, BindingId> = HashMap::new();
        for frame in &self.scope_stack.bindings_by_name {
            for (name, &id) in frame {
                visible.insert(name.clone(), id);
            }
        }
        let mut visible: Vec<(String, BindingId)> = visible.into_iter().collect();
        visible.sort_by(|a, b| a.0.cmp(&b.0));

        let mut inner = LoweringContext::new(self.config);
        inner.scope_stack.push_scope(&mut inner.arenas);
        for (name, id) in &visible {
            let outer = self.arenas.bindings.get(*id);
            let (kind, span) = (outer.kind, outer.introduced_at);
            inner.scope_stack.bind(&mut inner.arenas, name, kind, span);
        }
        let parts = inner.lower_regular_query(query);
        self.diagnostics.extend(inner.diagnostics);
        let query = HirQuery {
            arenas: inner.arenas,
            parts,
            diagnostics: Vec::new(),
        };
        (query, visible.into_iter().map(|(_, id)| id).collect())
    }

    fn lower_call_subquery(&mut self, c: &CallSubquery) {
        let (query, imported) = self.lower_subquery(&c.query);
        // The subquery's returned columns become variables of the outer query.
        let exported = c
            .query
            .returned_columns()
            .into_iter()
            .map(|(name, span)| {
                self.scope_stack
                    .bind(&mut self.arenas, &name, BindingKind::Value, span)
            })
            .collect();
        let in_transactions = c.in_transactions.as_ref().map(|it| HirInTransactions {
            of_rows: it.of_rows.as_ref().map(|e| self.lower_expr(e)),
            on_error: match it.on_error {
                Some(OnErrorBehavior::Continue) => HirOnErrorBehavior::Continue,
                Some(OnErrorBehavior::Break) => HirOnErrorBehavior::Break,
                Some(OnErrorBehavior::Fail) => HirOnErrorBehavior::Fail,
                None => HirOnErrorBehavior::Fail,
            },
        });
        self.current_part_ops
            .push(Operation::CallSubquery(CallSubqueryOp {
                imported_bindings: imported,
                exported_bindings: exported,
                query: Box::new(query),
                in_transactions,
            }));
    }

    fn lower_load_csv(&mut self, l: &LoadCsv) {
        let source = self.lower_expr(&l.source);
        let var = self.scope_stack.bind(
            &mut self.arenas,
            &l.variable.name.name,
            BindingKind::PatternBound,
            l.variable.name.span,
        );
        self.current_part_ops.push(Operation::LoadCsv(LoadCsvOp {
            source,
            variable: var,
            with_headers: l.with_headers,
            field_terminator: l.field_terminator.as_ref().map(|t| t.value.clone()),
        }));
    }

    fn lower_create(&mut self, c: &Create) {
        let pattern = self.lower_pattern(&c.pattern);
        self.current_part_ops
            .push(Operation::Create(CreateOp { pattern }));
    }

    fn lower_merge(&mut self, m: &Merge) {
        let pattern = self.lower_pattern_part_as_graph(&m.pattern);
        let mut on_create = Vec::new();
        let mut on_match = Vec::new();
        for action in &m.actions {
            let items: Vec<HirSetItem> = action
                .set_items
                .iter()
                .map(|si| self.lower_set_item(si))
                .collect();
            if action.on_match {
                on_match.extend(items);
            } else {
                on_create.extend(items);
            }
        }
        self.current_part_ops.push(Operation::Merge(MergeOp {
            pattern,
            on_create,
            on_match,
        }));
    }

    fn lower_delete(&mut self, d: &Delete) {
        let targets = d.targets.iter().map(|t| self.lower_expr(t)).collect();
        self.current_part_ops.push(Operation::Delete(DeleteOp {
            detach: d.detach,
            targets,
        }));
    }

    fn lower_set(&mut self, s: &Set) {
        let items: Vec<HirSetItem> = s.items.iter().map(|si| self.lower_set_item(si)).collect();
        self.current_part_ops.push(Operation::Set(SetOp { items }));
    }

    fn lower_set_item(&mut self, si: &SetItem) -> HirSetItem {
        match si {
            SetItem::DynamicProperty {
                property,
                key,
                value,
                ..
            } => {
                let entity = self.lower_expr(property);
                let key_id = self.lower_expr(key);
                let val = self.lower_expr(value);
                HirSetItem::SetDynamicProperty {
                    entity,
                    key: key_id,
                    value: val,
                }
            }
            SetItem::Property {
                property,
                value,
                operator,
            } => {
                let target = self.lower_expr(property);
                let val = self.lower_expr(value);
                match operator {
                    SetOperator::Assign => HirSetItem::SetProperty { target, value: val },
                    SetOperator::Add => HirSetItem::MergeProperties {
                        entity: Id(usize::MAX), // placeholder - need property base
                        value: val,
                    },
                }
            }
            SetItem::Variable {
                variable,
                value,
                operator,
            } => {
                let target = self.resolve_or_bind_variable(variable);
                let val = self.lower_expr(value);
                match operator {
                    SetOperator::Assign => HirSetItem::SetVariable { target, value: val },
                    SetOperator::Add => HirSetItem::MergeProperties {
                        entity: target,
                        value: val,
                    },
                }
            }
            SetItem::Labels { variable, labels } => {
                let node = self.resolve_or_bind_variable(variable);
                let label_ids: Vec<LabelId> = labels
                    .iter()
                    .map(|l| self.arenas.labels.intern(&l.name, Id))
                    .collect();
                HirSetItem::SetLabels {
                    node,
                    labels: label_ids,
                }
            }
        }
    }

    fn lower_remove(&mut self, r: &Remove) {
        let items: Vec<HirRemoveItem> = r
            .items
            .iter()
            .map(|ri| match ri {
                RemoveItem::Labels { variable, labels } => {
                    let node = self.resolve_or_bind_variable(variable);
                    let label_ids: Vec<LabelId> = labels
                        .iter()
                        .map(|l| self.arenas.labels.intern(&l.name, Id))
                        .collect();
                    HirRemoveItem::Labels {
                        node,
                        labels: label_ids,
                    }
                }
                RemoveItem::Property(expr) => HirRemoveItem::Property {
                    target: self.lower_expr(expr),
                },
            })
            .collect();
        self.current_part_ops
            .push(Operation::Remove(RemoveOp { items }));
    }

    fn lower_foreach(&mut self, f: &Foreach) {
        let list = self.lower_expr(&f.list);
        let var = self.scope_stack.bind(
            &mut self.arenas,
            &f.variable.name.name,
            BindingKind::ForeachVar,
            f.variable.name.span,
        );
        let mut ops = Vec::new();
        for update in &f.updates {
            match update {
                ForeachUpdate::Create(c) => {
                    let pattern = self.lower_pattern(&c.pattern);
                    ops.push(Operation::Create(CreateOp { pattern }));
                }
                ForeachUpdate::Merge(m) => {
                    let pattern = self.lower_pattern_part_as_graph(&m.pattern);
                    ops.push(Operation::Merge(MergeOp {
                        pattern,
                        on_create: Vec::new(),
                        on_match: Vec::new(),
                    }));
                }
                ForeachUpdate::Delete(d) => {
                    let targets = d.targets.iter().map(|t| self.lower_expr(t)).collect();
                    ops.push(Operation::Delete(DeleteOp {
                        detach: d.detach,
                        targets,
                    }));
                }
                ForeachUpdate::Set(s) => {
                    let items: Vec<HirSetItem> =
                        s.items.iter().map(|si| self.lower_set_item(si)).collect();
                    ops.push(Operation::Set(SetOp { items }));
                }
                ForeachUpdate::Remove(r) => {
                    let items: Vec<HirRemoveItem> = r
                        .items
                        .iter()
                        .map(|ri| match ri {
                            RemoveItem::Labels { variable, labels } => {
                                let node = self.resolve_or_bind_variable(variable);
                                let label_ids: Vec<LabelId> = labels
                                    .iter()
                                    .map(|l| self.arenas.labels.intern(&l.name, Id))
                                    .collect();
                                HirRemoveItem::Labels {
                                    node,
                                    labels: label_ids,
                                }
                            }
                            RemoveItem::Property(expr) => HirRemoveItem::Property {
                                target: self.lower_expr(expr),
                            },
                        })
                        .collect();
                    ops.push(Operation::Remove(RemoveOp { items }));
                }
                ForeachUpdate::Foreach(inner) => {
                    self.lower_foreach(inner);
                    // Collect any operations pushed
                }
            }
        }
        self.current_part_ops.push(Operation::Foreach(ForeachOp {
            variable: var,
            list,
            operations: ops,
        }));
    }

    fn lower_with(&mut self, w: &With) {
        // WITH acts as a scope boundary: expressions resolve to prior scope,
        // aliases introduce bindings in the new scope.
        if w.star {
            // Pass-through all visible bindings
        }

        self.lower_projection(&w.items, w.distinct, BindingKind::WithAlias);

        if let Some(where_expr) = &w.where_clause {
            let pred = self.lower_expr(where_expr);
            self.current_part_ops
                .push(Operation::Filter(FilterOp { predicate: pred }));
        }

        if let Some(order) = &w.order {
            let sort_items = order
                .items
                .iter()
                .map(|si| SortItem {
                    expression: self.lower_expr(&si.expression),
                    direction: match si.direction {
                        Some(crate::ast::clause::SortDirection::Ascending) => {
                            SortDirection::Ascending
                        }
                        Some(crate::ast::clause::SortDirection::Descending) => {
                            SortDirection::Descending
                        }
                        None => SortDirection::Ascending,
                    },
                })
                .collect();
            self.current_part_ops
                .push(Operation::Sort(SortOp { items: sort_items }));
        }

        if let Some(skip_expr) = &w.skip {
            let count = self.lower_expr(skip_expr);
            self.current_part_ops
                .push(Operation::Skip(SkipOp { count }));
        }

        if let Some(limit_expr) = &w.limit {
            let count = self.lower_expr(limit_expr);
            self.current_part_ops
                .push(Operation::Limit(LimitOp { count }));
        }
    }

    fn lower_return(&mut self, r: &Return) {
        if r.star {
            // Pass-through all visible bindings as projection items
            let visible: Vec<(String, BindingId)> = self
                .scope_stack
                .bindings_by_name
                .iter()
                .rev()
                .flat_map(|m| m.iter().map(|(k, &v)| (k.clone(), v)))
                .collect();
            let mut seen = std::collections::HashSet::new();
            let mut items = Vec::new();
            for (name, binding_id) in visible {
                if seen.insert(name.clone()) {
                    let expr_id = self.arenas.expressions.alloc(HirExpr {
                        kind: ExprKind::Binding(binding_id),
                        span: Span::new(0, 0),
                    });
                    items.push(ProjectionItem {
                        expression: expr_id,
                        alias: binding_id,
                    });
                }
            }
            self.current_part_ops.push(Operation::Project(ProjectOp {
                items,
                distinct: r.distinct,
            }));
        } else {
            self.lower_projection(&r.items, r.distinct, BindingKind::ReturnAlias);
        }

        if let Some(order) = &r.order {
            let sort_items = order
                .items
                .iter()
                .map(|si| SortItem {
                    expression: self.lower_expr(&si.expression),
                    direction: match si.direction {
                        Some(crate::ast::clause::SortDirection::Ascending) => {
                            SortDirection::Ascending
                        }
                        Some(crate::ast::clause::SortDirection::Descending) => {
                            SortDirection::Descending
                        }
                        None => SortDirection::Ascending,
                    },
                })
                .collect();
            self.current_part_ops
                .push(Operation::Sort(SortOp { items: sort_items }));
        }

        if let Some(skip_expr) = &r.skip {
            let count = self.lower_expr(skip_expr);
            self.current_part_ops
                .push(Operation::Skip(SkipOp { count }));
        }

        if let Some(limit_expr) = &r.limit {
            let count = self.lower_expr(limit_expr);
            self.current_part_ops
                .push(Operation::Limit(LimitOp { count }));
        }

        self.current_part_ops.push(Operation::Return(ReturnOp {
            items: Vec::new(), // already in Project/Aggregate
            distinct: r.distinct,
        }));
    }

    fn lower_standalone_call(&mut self, call: &crate::ast::procedure::StandaloneCall) -> Operation {
        let proc_op = self.lower_procedure_invocation(&call.call);
        if let Some(yield_spec) = &call.yield_items {
            match yield_spec {
                YieldSpec::Star { .. } => {}
                YieldSpec::Items(_yi) => {
                    // Already handled in lower_procedure_invocation
                }
            }
        }
        Operation::CallProcedure(proc_op)
    }

    fn lower_procedure_invocation(&mut self, proc: &ProcedureInvocation) -> CallProcedureOp {
        let key = proc
            .name
            .name
            .iter()
            .map(|s| s.name.replace('\x00', "\x00\x00"))
            .collect::<Vec<_>>()
            .join("\x00");
        let display = proc
            .name
            .name
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let procedure = self
            .arenas
            .functions
            .intern_with_display(&key, &display, Id);
        let args = proc
            .name
            .arguments
            .iter()
            .map(|a| self.lower_expr(a))
            .collect();
        CallProcedureOp {
            procedure,
            args,
            yields: Vec::new(),
        }
    }

    // ── Expression lowering ──────────────────────────────────────────────

    fn lower_expr(&mut self, expr: &Expression) -> ExprId {
        let kind = match expr {
            Expression::Literal(lit) => match lit {
                Literal::List(l) => {
                    let elements = l.elements.iter().map(|e| self.lower_expr(e)).collect();
                    ExprKind::List(elements)
                }
                Literal::Map(m) => {
                    let entries = m
                        .entries
                        .iter()
                        .map(|(key, val)| {
                            let property_key = self.arenas.property_keys.intern(&key.name.name, Id);
                            let expr_id = self.lower_expr(val);
                            (property_key, expr_id)
                        })
                        .collect();
                    ExprKind::Map(entries)
                }
                _ => ExprKind::Literal(self.lower_literal(lit)),
            },
            Expression::Variable(v) => match self.scope_stack.resolve(&v.name.name) {
                Some(binding_id) => ExprKind::Binding(binding_id),
                None => {
                    self.diagnostics.push(HirDiagnostic::UnknownVariable {
                        name: v.name.name.clone(),
                        span: v.name.span,
                    });
                    ExprKind::Binding(Id(usize::MAX))
                }
            },
            Expression::Parameter(p) => {
                let param_id = self.arenas.parameters.intern(&p.name.name, Id);
                ExprKind::Parameter(param_id)
            }
            Expression::PropertyLookup { base, property, .. } => {
                let base_id = self.lower_expr(base);
                let key_id = self.arenas.property_keys.intern(&property.name.name, Id);
                ExprKind::Property {
                    base: base_id,
                    key: key_id,
                }
            }
            Expression::NodeLabels { base, labels, .. } => {
                let base = self.lower_expr(base);
                match self.lower_node_labels(labels) {
                    (labels, None) => ExprKind::NodeLabels { base, labels },
                    (_, Some(expression)) => ExprKind::LabelCheck { base, expression },
                }
            }
            Expression::BinaryOp { op, lhs, rhs, .. } => {
                let left = self.lower_expr(lhs);
                let right = self.lower_expr(rhs);
                let hir_op = self.lower_binary_op(op);
                ExprKind::Binary {
                    op: hir_op,
                    left,
                    right,
                }
            }
            Expression::UnaryOp { op, operand, .. } => {
                let expr = self.lower_expr(operand);
                let hir_op = match op {
                    UnaryOperator::Negate => UnaryOp::Negate,
                    UnaryOperator::Plus => UnaryOp::Plus,
                    UnaryOperator::Not => UnaryOp::Not,
                };
                ExprKind::Unary { op: hir_op, expr }
            }
            Expression::Comparison { lhs, operators, .. } => {
                let left = self.lower_expr(lhs);
                let ops: Vec<(HirComparisonOperator, ExprId)> = operators
                    .iter()
                    .map(|(op, rhs)| {
                        let r = self.lower_expr(rhs);
                        let hir_op = self.lower_comparison_op(op);
                        (hir_op, r)
                    })
                    .collect();
                ExprKind::Comparison {
                    left,
                    operators: ops,
                }
            }
            Expression::ListIndex { list, index, .. } => {
                let list_id = self.lower_expr(list);
                let index_id = self.lower_expr(index);
                ExprKind::ListIndex {
                    list: list_id,
                    index: index_id,
                }
            }
            Expression::ListSlice {
                list, start, end, ..
            } => {
                let list_id = self.lower_expr(list);
                let start_id = start.as_ref().map(|s| self.lower_expr(s));
                let end_id = end.as_ref().map(|e| self.lower_expr(e));
                ExprKind::ListSlice {
                    list: list_id,
                    start: start_id,
                    end: end_id,
                }
            }
            Expression::In { lhs, rhs, .. } => {
                let left = self.lower_expr(lhs);
                let right = self.lower_expr(rhs);
                ExprKind::In {
                    lhs: left,
                    rhs: right,
                }
            }
            Expression::IsNull {
                operand, negated, ..
            } => {
                let expr = self.lower_expr(operand);
                ExprKind::IsNull {
                    operand: expr,
                    negated: *negated,
                }
            }
            Expression::IsNormalized {
                operand,
                negated,
                form,
                ..
            } => ExprKind::IsNormalized {
                operand: self.lower_expr(operand),
                negated: *negated,
                form: *form,
            },
            Expression::IsTyped {
                operand,
                negated,
                type_name,
                ..
            } => ExprKind::IsTyped {
                operand: self.lower_expr(operand),
                negated: *negated,
                type_name: type_name.clone(),
            },
            Expression::FunctionCall(fc) => {
                let key = Self::qualified_function_key(fc);
                let display = Self::qualified_function_name(fc);
                let func_id = self
                    .arenas
                    .functions
                    .intern_with_display(&key, &display, Id);
                let args = fc.arguments.iter().map(|a| self.lower_expr(a)).collect();
                ExprKind::FunctionCall {
                    function: func_id,
                    args,
                    distinct: fc.distinct,
                }
            }
            Expression::CountStar { .. } => ExprKind::CountStar,
            Expression::Case(case) => {
                let scrutinee = case.scrutinee.as_ref().map(|s| self.lower_expr(s));
                let alternatives = case
                    .alternatives
                    .iter()
                    .map(|alt| HirCaseAlternative {
                        when: self.lower_expr(&alt.when),
                        then: self.lower_expr(&alt.then),
                    })
                    .collect();
                let default = case.default.as_ref().map(|d| self.lower_expr(d));
                ExprKind::Case(HirCaseExpr {
                    scrutinee,
                    alternatives,
                    default,
                })
            }
            Expression::ListComprehension(lc) => {
                // The collection is evaluated in the enclosing scope.
                let collection = self.lower_expr(&lc.collection);
                let _scope = self.scope_stack.push_scope(&mut self.arenas);
                let var = self.scope_stack.bind(
                    &mut self.arenas,
                    &lc.variable.name.name,
                    BindingKind::ComprehensionVar,
                    lc.variable.name.span,
                );
                let filter = lc.filter.as_ref().map(|f| self.lower_expr(f));
                let map = lc.map.as_ref().map(|m| self.lower_expr(m));
                self.scope_stack.pop_scope();
                ExprKind::ListComprehension(HirListComprehension {
                    variable: var,
                    collection,
                    filter,
                    map,
                })
            }
            Expression::PatternComprehension(pc) => {
                // The path variable and the pattern's variables are local to
                // the comprehension.
                let _scope = self.scope_stack.push_scope(&mut self.arenas);
                let var = pc.variable.as_ref().map(|v| {
                    self.scope_stack.bind(
                        &mut self.arenas,
                        &v.name.name,
                        BindingKind::ComprehensionVar,
                        v.name.span,
                    )
                });
                let pattern = self.lower_relationships_pattern(&pc.pattern);
                let filter = pc.where_clause.as_ref().map(|w| self.lower_expr(w));
                let map = self.lower_expr(&pc.map);
                self.scope_stack.pop_scope();
                ExprKind::PatternComprehension(HirPatternComprehension {
                    variable: var,
                    pattern,
                    filter,
                    map,
                })
            }
            Expression::All(fe)
            | Expression::Any(fe)
            | Expression::None(fe)
            | Expression::Single(fe) => {
                let quantifier = match expr {
                    Expression::All(_) => CollectionQuantifier::All,
                    Expression::Any(_) => CollectionQuantifier::Any,
                    Expression::None(_) => CollectionQuantifier::None,
                    Expression::Single(_) => CollectionQuantifier::Single,
                    _ => unreachable!(),
                };
                // The collection is evaluated in the enclosing scope.
                let collection = self.lower_expr(&fe.collection);
                let _scope = self.scope_stack.push_scope(&mut self.arenas);
                let var = self.scope_stack.bind(
                    &mut self.arenas,
                    &fe.variable.name.name,
                    BindingKind::ComprehensionVar,
                    fe.variable.name.span,
                );
                let predicate = fe.predicate.as_ref().map(|p| self.lower_expr(p));
                self.scope_stack.pop_scope();
                ExprKind::CollectionFilter {
                    quantifier,
                    variable: var,
                    collection,
                    predicate,
                }
            }
            Expression::Reduce(r) => {
                // The initial value and the list are evaluated in the
                // enclosing scope.
                let init = self.lower_expr(&r.init);
                let list = self.lower_expr(&r.list);
                let _scope = self.scope_stack.push_scope(&mut self.arenas);
                let accumulator = self.scope_stack.bind(
                    &mut self.arenas,
                    &r.accumulator.name.name,
                    BindingKind::ComprehensionVar,
                    r.accumulator.name.span,
                );
                let variable = self.scope_stack.bind(
                    &mut self.arenas,
                    &r.variable.name.name,
                    BindingKind::ComprehensionVar,
                    r.variable.name.span,
                );
                let expression = self.lower_expr(&r.expression);
                self.scope_stack.pop_scope();
                ExprKind::Reduce {
                    accumulator,
                    init,
                    variable,
                    list,
                    expression,
                }
            }
            Expression::Parenthesized(inner) => {
                // Lower directly; parenthesization is syntactic
                return self.lower_expr(inner);
            }
            Expression::Pattern(rp) => {
                let pattern = self.lower_relationships_pattern(rp);
                ExprKind::PatternExpr(pattern)
            }
            Expression::Exists(exists) => {
                let inner = self.lower_exists_inner(&exists.inner);
                ExprKind::ExistsSubquery(inner)
            }
            Expression::CountSubquery(count) => {
                let inner = self.lower_count_subquery(&count.query);
                ExprKind::CountSubquery(inner)
            }
            Expression::CollectSubquery(collect) => {
                let inner = self.lower_collect_subquery(&collect.query);
                ExprKind::CollectSubquery(inner)
            }
            Expression::MapProjection(mp) => {
                let base = self.resolve_or_create_binding(&mp.base);
                let items = mp
                    .items
                    .iter()
                    .map(|item| match item {
                        MapProjectionItem::AllProperties { .. } => {
                            HirMapProjectionItem::AllProperties
                        }
                        MapProjectionItem::PropertyLookup { property } => {
                            let key = self.arenas.property_keys.intern(&property.name.name, Id);
                            HirMapProjectionItem::PropertyLookup { key }
                        }
                        MapProjectionItem::Literal { key, value } => {
                            let k = self.arenas.property_keys.intern(&key.name.name, Id);
                            let v = self.lower_expr(value);
                            HirMapProjectionItem::Literal { key: k, value: v }
                        }
                        // `{c}` is `{c: c}`.
                        MapProjectionItem::Variable { variable } => {
                            let k = self.arenas.property_keys.intern(&variable.name.name, Id);
                            let v = self.lower_expr(&Expression::Variable(variable.clone()));
                            HirMapProjectionItem::Literal { key: k, value: v }
                        }
                    })
                    .collect();
                ExprKind::MapProjection { base, items }
            }
        };

        let span = expr_span(expr);
        self.arenas.expressions.alloc(HirExpr { kind, span })
    }

    fn lower_literal(&mut self, lit: &Literal) -> HirLiteral {
        match lit {
            Literal::Number(n) => match n {
                NumberLiteral::Integer(i) => HirLiteral::Integer(*i),
                NumberLiteral::Float(f) => HirLiteral::Float(*f),
            },
            Literal::String(s) => HirLiteral::String(s.value.clone()),
            Literal::Boolean(b) => HirLiteral::Boolean(*b),
            Literal::Null => HirLiteral::Null,
            Literal::List(_) | Literal::Map(_) => {
                unreachable!(
                    "list/map literals are intercepted in lower_expr before lower_literal is called"
                )
            }
        }
    }

    fn lower_binary_op(&mut self, op: &BinaryOperator) -> BinaryOp {
        match op {
            BinaryOperator::Add => BinaryOp::Add,
            BinaryOperator::Subtract => BinaryOp::Subtract,
            BinaryOperator::Multiply => BinaryOp::Multiply,
            BinaryOperator::Divide => BinaryOp::Divide,
            BinaryOperator::Modulo => BinaryOp::Modulo,
            BinaryOperator::Power => BinaryOp::Power,
            BinaryOperator::And => BinaryOp::And,
            BinaryOperator::Or => BinaryOp::Or,
            BinaryOperator::Xor => BinaryOp::Xor,
        }
    }

    fn lower_comparison_op(&mut self, op: &ComparisonOperator) -> HirComparisonOperator {
        match op {
            ComparisonOperator::Eq => HirComparisonOperator::Eq,
            ComparisonOperator::Ne => HirComparisonOperator::Ne,
            ComparisonOperator::Lt => HirComparisonOperator::Lt,
            ComparisonOperator::Gt => HirComparisonOperator::Gt,
            ComparisonOperator::Le => HirComparisonOperator::Le,
            ComparisonOperator::Ge => HirComparisonOperator::Ge,
            ComparisonOperator::RegexMatch => HirComparisonOperator::RegexMatch,
            ComparisonOperator::StartsWith => HirComparisonOperator::StartsWith,
            ComparisonOperator::EndsWith => HirComparisonOperator::EndsWith,
            ComparisonOperator::Contains => HirComparisonOperator::Contains,
        }
    }

    fn lower_exists_inner(&mut self, inner: &ExistsInner) -> ExistsSubquery {
        match inner {
            ExistsInner::Pattern(pat, where_clause) => {
                // Convert pattern+where to a minimal query. Variables the
                // pattern introduces are local to the predicate.
                let scope = self.scope_stack.push_scope(&mut self.arenas);
                let pattern = self.lower_pattern(pat);
                let mut ops = vec![Operation::Match(MatchOp {
                    pattern,
                    predicates: Vec::new(),
                })];
                if let Some(w) = where_clause {
                    let pred = self.lower_expr(w);
                    ops.push(Operation::Filter(FilterOp { predicate: pred }));
                }
                ops.push(Operation::Return(ReturnOp {
                    items: Vec::new(),
                    distinct: false,
                }));
                let out_scope = self.scope_stack.push_scope(&mut self.arenas);
                let query = HirQuery {
                    arenas: HirArenas::new(),
                    parts: vec![QueryPart {
                        input_scope: scope,
                        operations: ops,
                        output_scope: out_scope,
                    }],
                    diagnostics: Vec::new(),
                };
                self.scope_stack.pop_scope();
                self.scope_stack.pop_scope();
                ExistsSubquery {
                    query: Box::new(query),
                    imported_bindings: Vec::new(),
                }
            }
            ExistsInner::RegularQuery(rq) => {
                let (query, imported) = self.lower_subquery(rq);
                ExistsSubquery {
                    query: Box::new(query),
                    imported_bindings: imported,
                }
            }
        }
    }

    fn lower_count_subquery(&mut self, query: &RegularQuery) -> CountSubquery {
        let (q, imported) = self.lower_subquery(query);
        CountSubquery {
            query: Box::new(q),
            imported_bindings: imported,
        }
    }

    fn lower_collect_subquery(&mut self, query: &RegularQuery) -> CollectSubquery {
        let (q, imported) = self.lower_subquery(query);
        CollectSubquery {
            query: Box::new(q),
            imported_bindings: imported,
        }
    }

    // ── Pattern lowering ─────────────────────────────────────────────────

    fn lower_pattern(&mut self, pattern: &Pattern) -> GraphPattern {
        self.lower_pattern_parts(&pattern.parts)
    }

    /// Lowers pattern parts, binding each `p = …` path variable.
    fn lower_pattern_parts(&mut self, parts: &[crate::ast::pattern::PatternPart]) -> GraphPattern {
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();
        let mut path_bindings = Vec::new();

        for part in parts {
            let path_binding = part.variable.as_ref().map(|v| {
                self.scope_stack.bind(
                    &mut self.arenas,
                    &v.name.name,
                    BindingKind::PatternBound,
                    v.name.span,
                )
            });

            let (part_nodes, part_rels) = self.lower_pattern_element(&part.anonymous.element);
            let node_start = nodes.len();
            nodes.extend(part_nodes);
            let rel_start = relationships.len();
            relationships.extend(part_rels);

            if let Some(binding_id) = path_binding {
                let node_indices: Vec<NodeIndex> = (node_start..nodes.len()).collect();
                let rel_indices: Vec<RelIndex> = (rel_start..relationships.len()).collect();
                path_bindings.push(PathBinding {
                    binding: binding_id,
                    nodes: node_indices,
                    relationships: rel_indices,
                });
            }
        }

        GraphPattern {
            nodes,
            relationships,
            path_bindings,
        }
    }

    fn lower_pattern_element(
        &mut self,
        element: &PatternElement,
    ) -> (Vec<HirNodePattern>, Vec<HirRelationshipPattern>) {
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();

        match element {
            PatternElement::Path { start, chains } => {
                let start_idx = nodes.len();
                let start_node = self.lower_node_pattern(start);
                nodes.push(start_node);
                let mut current = start_idx;

                for chain in chains {
                    let end_node = self.lower_node_pattern(&chain.node);
                    let end_idx = nodes.len();
                    nodes.push(end_node);

                    let rel =
                        self.lower_relationship_pattern(&chain.relationship, current, end_idx);
                    relationships.push(rel);
                    current = end_idx;
                }
            }
            PatternElement::Parenthesized(inner) => {
                return self.lower_pattern_element(inner);
            }
            PatternElement::Quantified { element, .. } => {
                // For now, lower the inner element without the quantifier
                return self.lower_pattern_element(element);
            }
        }

        (nodes, relationships)
    }

    /// Binding for a node or relationship variable in a pattern: a variable
    /// that is already visible refers to the same entity (`MATCH (n) MATCH
    /// (n)-->(m)`), so its binding is reused; otherwise a new one is created.
    fn bind_pattern_variable(&mut self, v: &Variable) -> BindingId {
        match self.scope_stack.resolve(&v.name.name) {
            Some(id) => id,
            None => self.scope_stack.bind(
                &mut self.arenas,
                &v.name.name,
                BindingKind::PatternBound,
                v.name.span,
            ),
        }
    }

    fn lower_node_pattern(&mut self, node: &NodePattern) -> HirNodePattern {
        let binding = node
            .variable
            .as_ref()
            .map(|v| self.bind_pattern_variable(v));
        let (labels, label_expression) = self.lower_node_labels(&node.labels);
        let properties = node.properties.as_ref().map(|p| self.lower_properties(p));
        HirNodePattern {
            binding,
            labels,
            label_expression,
            properties,
        }
    }

    fn lower_relationship_pattern(
        &mut self,
        rel: &RelationshipPattern,
        left: NodeIndex,
        right: NodeIndex,
    ) -> HirRelationshipPattern {
        let (binding, types, type_expression, length, properties) =
            if let Some(detail) = &rel.detail {
                let binding = detail
                    .variable
                    .as_ref()
                    .map(|v| self.bind_pattern_variable(v));
                let (types, type_expression) = self.lower_rel_types(detail.types.as_ref());
                let length = detail
                    .range
                    .as_ref()
                    .map(|r| RelationshipLength::Variable {
                        min: r.start.map(|n| n as u32),
                        max: r.end.map(|n| n as u32),
                    })
                    .unwrap_or(RelationshipLength::Single);
                let properties = detail.properties.as_ref().map(|p| self.lower_properties(p));
                (binding, types, type_expression, length, properties)
            } else {
                (None, Vec::new(), None, RelationshipLength::Single, None)
            };

        let direction = match rel.direction {
            RelationshipDirection::Left => HirRelationshipDirection::RightToLeft,
            RelationshipDirection::Right => HirRelationshipDirection::LeftToRight,
            RelationshipDirection::Both => HirRelationshipDirection::Both,
            RelationshipDirection::Undirected => HirRelationshipDirection::Undirected,
        };

        HirRelationshipPattern {
            binding,
            type_expression,
            direction,
            left,
            right,
            types,
            length,
            properties,
        }
    }

    fn lower_relationships_pattern(
        &mut self,
        rp: &crate::ast::pattern::RelationshipsPattern,
    ) -> GraphPattern {
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();

        let start_idx = nodes.len();
        nodes.push(self.lower_node_pattern(&rp.start));
        let mut current = start_idx;

        for chain in &rp.chains {
            let end_idx = nodes.len();
            nodes.push(self.lower_node_pattern(&chain.node));
            let rel = self.lower_relationship_pattern(&chain.relationship, current, end_idx);
            relationships.push(rel);
            current = end_idx;
        }

        GraphPattern {
            nodes,
            relationships,
            path_bindings: Vec::new(),
        }
    }

    fn lower_pattern_part_as_graph(
        &mut self,
        part: &crate::ast::pattern::PatternPart,
    ) -> GraphPattern {
        // `MERGE p = (a)-[:R]->(b)` binds `p` just like `MATCH p = …`.
        self.lower_pattern_parts(std::slice::from_ref(part))
    }

    /// Lowers a node-label (`for_types == false`) or relationship-type
    /// (`for_types == true`) expression.
    fn lower_label_expr(&mut self, expr: &LabelExpression, for_types: bool) -> HirLabelExpr {
        match expr {
            LabelExpression::Static(sym) => HirLabelExpr::Name(if for_types {
                self.arenas.relationship_types.intern(&sym.name, Id)
            } else {
                self.arenas.labels.intern(&sym.name, Id)
            }),
            LabelExpression::Wildcard { .. } => HirLabelExpr::Wildcard,
            LabelExpression::Dynamic { expression, .. } => {
                HirLabelExpr::Dynamic(self.lower_expr(expression))
            }
            LabelExpression::Or { lhs, rhs, .. } => HirLabelExpr::Or(
                Box::new(self.lower_label_expr(lhs, for_types)),
                Box::new(self.lower_label_expr(rhs, for_types)),
            ),
            LabelExpression::And { lhs, rhs, .. } => HirLabelExpr::And(
                Box::new(self.lower_label_expr(lhs, for_types)),
                Box::new(self.lower_label_expr(rhs, for_types)),
            ),
            LabelExpression::Not { inner, .. } => {
                HirLabelExpr::Not(Box::new(self.lower_label_expr(inner, for_types)))
            }
            LabelExpression::Group { inner, .. } => self.lower_label_expr(inner, for_types),
        }
    }

    /// Lowers the label groups of a node pattern or label predicate (`:A:B`
    /// is two groups, all required). A conjunction of static labels becomes a
    /// label list; anything else a label expression.
    fn lower_node_labels(
        &mut self,
        groups: &[LabelExpression],
    ) -> (Vec<LabelId>, Option<HirLabelExpr>) {
        fn static_conjunction(expr: &LabelExpression, out: &mut Vec<String>) -> bool {
            match expr {
                LabelExpression::Static(sym) => {
                    out.push(sym.name.clone());
                    true
                }
                LabelExpression::And { lhs, rhs, .. } => {
                    static_conjunction(lhs, out) && static_conjunction(rhs, out)
                }
                LabelExpression::Group { inner, .. } => static_conjunction(inner, out),
                _ => false,
            }
        }
        let mut names = Vec::new();
        if groups.iter().all(|g| static_conjunction(g, &mut names)) {
            let ids = names
                .iter()
                .map(|n| self.arenas.labels.intern(n, Id))
                .collect();
            return (ids, None);
        }
        let expression = groups
            .iter()
            .map(|g| self.lower_label_expr(g, false))
            .reduce(|acc, next| HirLabelExpr::And(Box::new(acc), Box::new(next)));
        (Vec::new(), expression)
    }

    /// Lowers a relationship type expression. An alternation of static types
    /// (`:A|B`) becomes a type list; anything else a type expression.
    fn lower_rel_types(
        &mut self,
        expr: Option<&LabelExpression>,
    ) -> (Vec<RelTypeId>, Option<HirLabelExpr>) {
        fn static_alternation(expr: &LabelExpression, out: &mut Vec<String>) -> bool {
            match expr {
                LabelExpression::Static(sym) => {
                    out.push(sym.name.clone());
                    true
                }
                LabelExpression::Or { lhs, rhs, .. } => {
                    static_alternation(lhs, out) && static_alternation(rhs, out)
                }
                LabelExpression::Group { inner, .. } => static_alternation(inner, out),
                _ => false,
            }
        }
        let Some(expr) = expr else {
            return (Vec::new(), None);
        };
        let mut names = Vec::new();
        if static_alternation(expr, &mut names) {
            let ids = names
                .iter()
                .map(|n| self.arenas.relationship_types.intern(n, Id))
                .collect();
            return (ids, None);
        }
        (Vec::new(), Some(self.lower_label_expr(expr, true)))
    }

    fn lower_properties(&mut self, props: &Properties) -> ExprId {
        match props {
            Properties::Map(m) => {
                let entries: Vec<(PropertyKeyId, ExprId)> = m
                    .entries
                    .iter()
                    .map(|(key, val)| {
                        let k = self.arenas.property_keys.intern(&key.name.name, Id);
                        let v = self.lower_expr(val);
                        (k, v)
                    })
                    .collect();
                let kind = ExprKind::Map(entries);
                self.arenas
                    .expressions
                    .alloc(HirExpr { kind, span: m.span })
            }
            Properties::Parameter(p) => {
                let param_id = self.arenas.parameters.intern(&p.name.name, Id);
                let kind = ExprKind::Parameter(param_id);
                self.arenas
                    .expressions
                    .alloc(HirExpr { kind, span: p.span })
            }
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    /// Resolves the target of an update or projection (`SET n:L`,
    /// `SET n = …`, `n {.a}`), which must already be bound. An unknown name is
    /// reported, then bound so that lowering can continue.
    fn resolve_or_bind_variable(&mut self, var: &Variable) -> BindingId {
        match self.scope_stack.resolve(&var.name.name) {
            Some(id) => id,
            None => {
                self.diagnostics.push(HirDiagnostic::UnknownVariable {
                    name: var.name.name.clone(),
                    span: var.name.span,
                });
                self.scope_stack.bind(
                    &mut self.arenas,
                    &var.name.name,
                    BindingKind::Value,
                    var.name.span,
                )
            }
        }
    }

    fn resolve_or_create_binding(&mut self, var: &Variable) -> BindingId {
        self.resolve_or_bind_variable(var)
    }

    fn qualified_function_name(fc: &FunctionInvocation) -> String {
        let mut name = String::new();
        for (idx, segment) in fc.name.iter().enumerate() {
            if idx > 0 {
                name.push('.');
            }
            name.push_str(&segment.name);
        }
        name
    }

    fn qualified_function_key(fc: &FunctionInvocation) -> String {
        fc.name
            .iter()
            .map(|s| s.name.replace('\x00', "\x00\x00"))
            .collect::<Vec<_>>()
            .join("\x00")
    }

    fn infer_alias_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Variable(v) => v.name.name.clone(),
            Expression::PropertyLookup { property, .. } => property.name.name.clone(),
            Expression::FunctionCall(fc) => {
                let name = Self::qualified_function_name(fc);
                if name.is_empty() {
                    "expr".to_string()
                } else {
                    name
                }
            }
            _ => "expr".to_string(),
        }
    }

    fn is_aggregate_call(&self, fc: &crate::ast::expr::FunctionInvocation) -> bool {
        self.config
            .aggregates
            .contains(&Self::qualified_function_name(fc))
    }

    fn has_aggregate(&self, expr: &Expression) -> bool {
        expr.contains_aggregate(&|fc| self.is_aggregate_call(fc))
    }

    /// Lowers the items of a `WITH` or `RETURN` projection, binding each
    /// item's alias with `kind`.
    ///
    /// Without aggregates this is a single `Project`. With aggregates, an
    /// `Aggregate` operation computes the grouping keys and every aggregate
    /// call, including calls nested in larger expressions (`abs(sum(x))`,
    /// `count(n) + 1`), each into its own binding. A following `Project`
    /// then computes those larger expressions from the aggregate bindings
    /// and applies `DISTINCT`.
    fn lower_projection(
        &mut self,
        items: &[crate::ast::clause::ProjectionItem],
        distinct: bool,
        kind: BindingKind,
    ) {
        self.report_duplicate_columns(items);

        let alias_of = |this: &Self, pi: &crate::ast::clause::ProjectionItem| match &pi.alias {
            Some(a) => (a.name.name.clone(), a.name.span),
            None => (
                this.infer_alias_name(&pi.expression),
                expr_span(&pi.expression),
            ),
        };

        if !items.iter().any(|pi| self.has_aggregate(&pi.expression)) {
            // Evaluate every expression before any alias becomes visible.
            let exprs: Vec<ExprId> = items
                .iter()
                .map(|pi| self.lower_expr(&pi.expression))
                .collect();
            let items = items
                .iter()
                .zip(exprs)
                .map(|(pi, expression)| {
                    let (name, span) = alias_of(self, pi);
                    let alias = self.scope_stack.bind(&mut self.arenas, &name, kind, span);
                    ProjectionItem { expression, alias }
                })
                .collect();
            self.current_part_ops
                .push(Operation::Project(ProjectOp { items, distinct }));
            return;
        }

        enum Column {
            Key(ExprId),
            Aggregate(FunctionId, Vec<ExprId>, bool),
            /// An expression around aggregates, rewritten to reference the
            /// synthetic bindings of the extracted aggregate calls.
            Computed(Expression),
        }
        let mut columns = Vec::new();
        // Extracted nested aggregate calls, bound under unspellable names.
        let mut nested: Vec<(String, FunctionId, Vec<ExprId>, bool, Span)> = Vec::new();
        for pi in items {
            let column = if !self.has_aggregate(&pi.expression) {
                Column::Key(self.lower_expr(&pi.expression))
            } else if let Some((function, args, d)) = self.lower_aggregate_call(&pi.expression) {
                Column::Aggregate(function, args, d)
            } else {
                let mut rewritten = pi.expression.clone();
                let offset = nested.len();
                let calls = rewritten.extract_aggregates(&|fc| self.is_aggregate_call(fc), &|i| {
                    format!(" aggregate {}", offset + i)
                });
                for call in calls {
                    let span = expr_span(&call);
                    let name = format!(" aggregate {}", nested.len());
                    let (function, args, d) = self
                        .lower_aggregate_call(&call)
                        .expect("extracted expression is an aggregate call");
                    nested.push((name, function, args, d, span));
                }
                Column::Computed(rewritten)
            };
            columns.push((column, alias_of(self, pi)));
        }

        let mut grouping_keys = Vec::new();
        let mut aggregates = Vec::new();
        let mut computed = Vec::new();
        let mut outputs = Vec::new();
        for (column, (name, span)) in columns {
            match column {
                Column::Key(expression) => {
                    let alias = self.scope_stack.bind(&mut self.arenas, &name, kind, span);
                    grouping_keys.push(ProjectionItem { expression, alias });
                    outputs.push(Some(alias));
                }
                Column::Aggregate(function, args, distinct) => {
                    let alias = self.scope_stack.bind(&mut self.arenas, &name, kind, span);
                    aggregates.push(AggregateItem {
                        function,
                        args,
                        distinct,
                        alias,
                    });
                    outputs.push(Some(alias));
                }
                Column::Computed(expr) => {
                    computed.push((expr, name, span));
                    outputs.push(None);
                }
            }
        }

        // The synthetic bindings are only visible while lowering the
        // computed expressions.
        self.scope_stack.push_scope(&mut self.arenas);
        for (name, function, args, distinct, span) in nested {
            let alias = self
                .scope_stack
                .bind(&mut self.arenas, &name, BindingKind::Value, span);
            aggregates.push(AggregateItem {
                function,
                args,
                distinct,
                alias,
            });
        }
        let computed: Vec<_> = computed
            .into_iter()
            .map(|(expr, name, span)| (self.lower_expr(&expr), name, span))
            .collect();
        self.scope_stack.pop_scope();

        let needs_projection = distinct || !computed.is_empty();
        self.current_part_ops
            .push(Operation::Aggregate(AggregateOp {
                grouping_keys,
                aggregates,
            }));
        if !needs_projection {
            return;
        }

        let mut computed = computed.into_iter();
        let items = outputs
            .into_iter()
            .map(|output| match output {
                Some(alias) => ProjectionItem {
                    expression: self.arenas.expressions.alloc(HirExpr {
                        kind: ExprKind::Binding(alias),
                        span: Span::new(0, 0),
                    }),
                    alias,
                },
                None => {
                    let (expression, name, span) =
                        computed.next().expect("one computed expression per column");
                    let alias = self.scope_stack.bind(&mut self.arenas, &name, kind, span);
                    ProjectionItem { expression, alias }
                }
            })
            .collect();
        self.current_part_ops
            .push(Operation::Project(ProjectOp { items, distinct }));
    }

    /// Lowers `expr` if it is itself an aggregate call (`count(*)` or a
    /// registered aggregate function), returning the function, its arguments
    /// and whether it is `DISTINCT`.
    fn lower_aggregate_call(
        &mut self,
        expr: &Expression,
    ) -> Option<(FunctionId, Vec<ExprId>, bool)> {
        match expr {
            Expression::CountStar { .. } => {
                Some((self.arenas.functions.intern("COUNT", Id), Vec::new(), false))
            }
            Expression::FunctionCall(fc) if self.is_aggregate_call(fc) => {
                let key = Self::qualified_function_key(fc);
                let display = Self::qualified_function_name(fc);
                let function = self
                    .arenas
                    .functions
                    .intern_with_display(&key, &display, Id);
                let args = fc.arguments.iter().map(|a| self.lower_expr(a)).collect();
                Some((function, args, fc.distinct))
            }
            Expression::Parenthesized(inner) => self.lower_aggregate_call(inner),
            _ => None,
        }
    }

    /// Reports projection items that produce the same column name
    /// (`RETURN 1 AS a, 2 AS a`, `RETURN count(*), count(*)`).
    fn report_duplicate_columns(&mut self, items: &[crate::ast::clause::ProjectionItem]) {
        let mut seen: HashMap<String, Span> = HashMap::new();
        for pi in items {
            let (name, span) = pi.column();
            if let Some(&first) = seen.get(&name) {
                self.diagnostics.push(HirDiagnostic::DuplicateVariable {
                    name,
                    first,
                    second: span,
                });
            } else {
                seen.insert(name, span);
            }
        }
    }
}

fn expr_span(expr: &Expression) -> Span {
    expr.span()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::LowerConfig;
    use crate::parse;

    #[test]
    fn test_simple_match_return() {
        let query = parse("MATCH (p:Person) RETURN p.name").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        assert_eq!(hir.parts.len(), 1);
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
        assert!(matches!(&ops[2], Operation::Return(_)));
    }

    #[test]
    fn test_with_aggregation() {
        let query = parse("MATCH (p:Person) WITH count(*) AS n RETURN n").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        // WITH creates a new query part, so we get 2 parts
        assert_eq!(hir.parts.len(), 2);
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Aggregate(_)));
    }

    #[test]
    fn test_multi_part_query() {
        let query = parse("MATCH (p:Person) WITH p MATCH (p)-[:KNOWS]->(f) RETURN f.name").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        assert_eq!(hir.parts.len(), 2);
    }

    #[test]
    fn test_unknown_variable_error() {
        let query = parse("MATCH (p:Person) RETURN x.name").unwrap();
        let result = lower(&query, &LowerConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_where_after_match() {
        let query = parse("MATCH (p:Person) WHERE p.age > 18 RETURN p.name").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        match &ops[0] {
            Operation::Match(m) => {
                assert!(!m.predicates.is_empty());
            }
            _ => panic!("Expected MatchOp"),
        }
    }

    #[test]
    fn test_where_after_with() {
        let query = parse("MATCH (p:Person) WITH p WHERE p.age > 18 RETURN p.name").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
        assert!(matches!(&ops[2], Operation::Filter(_)));
    }

    #[test]
    fn test_optional_match() {
        let query = parse("OPTIONAL MATCH (p:Person) RETURN p.name").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::OptionalMatch(_)));
    }

    #[test]
    fn test_create_clause() {
        let query = parse("CREATE (p:Person {name: 'Alice'})").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Create(_)));
    }

    #[test]
    fn test_delete_clause() {
        let query = parse("MATCH (p:Person) DELETE p").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Delete(_)));
    }

    #[test]
    fn test_unwind_clause() {
        let query = parse("UNWIND [1, 2, 3] AS x RETURN x").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Unwind(_)));
    }

    #[test]
    fn test_return_star() {
        let query = parse("MATCH (p:Person) RETURN *").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
    }

    #[test]
    fn test_order_by_and_limit() {
        let query = parse("MATCH (p:Person) RETURN p.name ORDER BY p.name ASC LIMIT 10").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
        assert!(matches!(&ops[2], Operation::Sort(_)));
        assert!(matches!(&ops[3], Operation::Limit(_)));
        assert!(matches!(&ops[4], Operation::Return(_)));
    }

    #[test]
    fn test_function_call_preserves_qualified_name() {
        let query = parse("RETURN apoc.text.distance('hello', 'world') AS d").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;

        let function = match &ops[0] {
            Operation::Project(ProjectOp { items, .. }) => {
                match &hir.arenas.expressions.get(items[0].expression).kind {
                    ExprKind::FunctionCall { function, .. } => *function,
                    _ => panic!("Expected function call expression"),
                }
            }
            _ => panic!("Expected ProjectOp"),
        };

        assert_eq!(
            hir.arenas.functions.name_of(function),
            Some("apoc.text.distance")
        );
    }

    #[test]
    fn test_infer_alias_name_uses_full_qualified_name() {
        let query = parse("RETURN apoc.text.distance('hello', 'world')").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;

        let alias_id = match &ops[0] {
            Operation::Project(ProjectOp { items, .. }) => items[0].alias,
            _ => panic!("Expected ProjectOp"),
        };

        assert_eq!(hir.arenas.bindings.get(alias_id).name, "apoc.text.distance");
    }

    #[test]
    fn test_qualified_function_name_does_not_collide_with_builtin() {
        let query = parse(
            "RETURN replace('hello', 'l', 'x') AS a, apoc.text.replace('hello', 'l', 'x') AS b",
        )
        .unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;

        let (builtin, qualified) = match &ops[0] {
            Operation::Project(ProjectOp { items, .. }) => {
                let builtin = match &hir.arenas.expressions.get(items[0].expression).kind {
                    ExprKind::FunctionCall { function, .. } => *function,
                    _ => panic!("Expected first expression to be a function call"),
                };
                let qualified = match &hir.arenas.expressions.get(items[1].expression).kind {
                    ExprKind::FunctionCall { function, .. } => *function,
                    _ => panic!("Expected second expression to be a function call"),
                };
                (builtin, qualified)
            }
            _ => panic!("Expected ProjectOp"),
        };

        assert_ne!(builtin, qualified);
        assert_eq!(hir.arenas.functions.name_of(builtin), Some("replace"));
        assert_eq!(
            hir.arenas.functions.name_of(qualified),
            Some("apoc.text.replace")
        );
    }

    #[test]
    fn test_namespaced_count_is_not_aggregate() {
        // apoc.coll.count has "count" as its last segment but is NOT in the default
        // aggregate registry. It must lower to ProjectOp, not AggregateOp.
        let query = parse("MATCH (n) WITH apoc.coll.count(n.name) AS c RETURN c").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        assert_eq!(hir.parts.len(), 2);
        let ops = &hir.parts[0].operations;
        assert!(
            matches!(&ops[1], Operation::Project(_)),
            "expected ProjectOp but got: {:?}",
            ops[1]
        );
    }

    #[test]
    fn test_custom_registered_aggregate() {
        // A custom qualified aggregate registered in LowerConfig must be routed
        // to AggregateOp, not ProjectOp.
        let mut config = LowerConfig::default();
        config.aggregates.register("apoc.agg.percentiles");

        let query = parse("MATCH (n) WITH apoc.agg.percentiles(n.value) AS p RETURN p").unwrap();
        let hir = lower(&query, &config).unwrap();
        assert_eq!(hir.parts.len(), 2);
        let ops = &hir.parts[0].operations;
        assert!(
            matches!(&ops[1], Operation::Aggregate(_)),
            "expected AggregateOp but got: {:?}",
            ops[1]
        );
    }

    #[test]
    fn test_escaped_segment_does_not_collide_with_dotted_namespace() {
        // `apoc.text`.distance has 2 segments: ["apoc.text", "distance"]
        // apoc.text.distance has 3 segments: ["apoc", "text", "distance"]
        // They must produce distinct FunctionIds.
        let query =
            parse("RETURN `apoc.text`.distance('a', 'b'), apoc.text.distance('a', 'b')").unwrap();
        let hir = lower(&query, &LowerConfig::default()).unwrap();
        let ops = &hir.parts[0].operations;

        let (escaped_id, plain_id) = match &ops[0] {
            Operation::Project(ProjectOp { items, .. }) => {
                let escaped = match &hir.arenas.expressions.get(items[0].expression).kind {
                    ExprKind::FunctionCall { function, .. } => *function,
                    _ => panic!("Expected FunctionCall for item 0"),
                };
                let plain = match &hir.arenas.expressions.get(items[1].expression).kind {
                    ExprKind::FunctionCall { function, .. } => *function,
                    _ => panic!("Expected FunctionCall for item 1"),
                };
                (escaped, plain)
            }
            _ => panic!("Expected ProjectOp"),
        };

        assert_ne!(
            escaped_id, plain_id,
            "`apoc.text`.distance and apoc.text.distance must have distinct FunctionIds"
        );
    }
}
