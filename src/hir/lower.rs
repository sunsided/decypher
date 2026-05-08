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
    BinaryOperator, ComparisonOperator, ExistsInner, Expression, Literal, MapProjectionItem,
    NumberLiteral, UnaryOperator,
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

use super::arena::{BindingId, ExprId, Id, LabelId, PropertyKeyId, RelTypeId, ScopeId};
use super::binding::{Binding, BindingKind, Scope};
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
    OnErrorBehavior as HirOnErrorBehavior, Operation, ProjectOp, ProjectionItem,
    RemoveItem as HirRemoveItem, RemoveOp, ReturnOp, SetItem as HirSetItem, SetOp, SkipOp,
    SortDirection, SortItem, SortOp, UnwindOp,
};
use super::pattern::{
    GraphPattern, NodeIndex, NodePattern as HirNodePattern, PathBinding, RelIndex,
    RelationshipDirection as HirRelationshipDirection, RelationshipLength,
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
pub fn lower(query: &Query) -> Result<HirQuery, Diagnostics> {
    let mut ctx = LoweringContext::new();
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
struct LoweringContext {
    arenas: HirArenas,
    diagnostics: Vec<HirDiagnostic>,
    scope_stack: LoweringScopeStack,
    current_part_ops: Vec<Operation>,
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

impl LoweringContext {
    /// Create a fresh lowering context with empty arenas and scope stack.
    fn new() -> Self {
        Self {
            arenas: HirArenas::new(),
            diagnostics: Vec::new(),
            scope_stack: LoweringScopeStack::new(),
            current_part_ops: Vec::new(),
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
        let proc_op = self.lower_procedure_invocation(&i.call);
        self.current_part_ops
            .push(Operation::CallProcedure(proc_op));
    }

    fn lower_call_subquery(&mut self, c: &CallSubquery) {
        let imported = self
            .scope_stack
            .bindings_by_name
            .iter()
            .rev()
            .flat_map(|m| m.values())
            .copied()
            .collect();
        let query_ast = Query {
            statements: vec![QueryBody::Regular(c.query.clone())],
            span: Span::new(0, 0),
        };
        let query = lower(&query_ast).unwrap_or_else(|_| HirQuery {
            arenas: HirArenas::new(),
            parts: Vec::new(),
            diagnostics: Vec::new(),
        });
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

        let mut items: Vec<ProjectionItem> = Vec::new();
        for pi in &w.items {
            let expr = self.lower_expr(&pi.expression);
            let alias_name = pi
                .alias
                .as_ref()
                .map(|v| v.name.name.clone())
                .unwrap_or_else(|| self.infer_alias_name(&pi.expression));
            let alias = self.scope_stack.bind(
                &mut self.arenas,
                &alias_name,
                BindingKind::WithAlias,
                expr_span(&pi.expression),
            );
            items.push(ProjectionItem {
                expression: expr,
                alias,
            });
        }

        // Check for aggregates
        let has_aggregate = w.items.iter().any(|pi| self.has_aggregate(&pi.expression));
        if has_aggregate {
            let mut grouping_keys = Vec::new();
            let mut aggregates = Vec::new();
            for pi in &w.items {
                if self.has_aggregate(&pi.expression) {
                    let (func_id, args, distinct) = match &pi.expression {
                        Expression::FunctionCall(fc) => {
                            let name = fc.name.last().map(|s| s.name.clone()).unwrap_or_default();
                            let fid = self.arenas.functions.intern(&name, Id);
                            let a = fc.arguments.iter().map(|a| self.lower_expr(a)).collect();
                            (fid, a, fc.distinct)
                        }
                        Expression::CountStar { .. } => {
                            let fid = self.arenas.functions.intern("COUNT", Id);
                            (fid, vec![], false)
                        }
                        _ => {
                            let fid = self.arenas.functions.intern("UNKNOWN", Id);
                            (fid, vec![], false)
                        }
                    };
                    let alias_name = pi
                        .alias
                        .as_ref()
                        .map(|v| v.name.name.clone())
                        .unwrap_or_else(|| self.infer_alias_name(&pi.expression));
                    let alias = self.scope_stack.bind(
                        &mut self.arenas,
                        &alias_name,
                        BindingKind::WithAlias,
                        expr_span(&pi.expression),
                    );
                    aggregates.push(AggregateItem {
                        function: func_id,
                        args,
                        distinct,
                        alias,
                    });
                } else {
                    let expr = self.lower_expr(&pi.expression);
                    let alias_name = pi
                        .alias
                        .as_ref()
                        .map(|v| v.name.name.clone())
                        .unwrap_or_else(|| self.infer_alias_name(&pi.expression));
                    let alias = self.scope_stack.bind(
                        &mut self.arenas,
                        &alias_name,
                        BindingKind::WithAlias,
                        expr_span(&pi.expression),
                    );
                    grouping_keys.push(ProjectionItem {
                        expression: expr,
                        alias,
                    });
                }
            }
            self.current_part_ops
                .push(Operation::Aggregate(AggregateOp {
                    grouping_keys,
                    aggregates,
                }));
        } else {
            self.current_part_ops.push(Operation::Project(ProjectOp {
                items,
                distinct: w.distinct,
            }));
        }

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
            let mut items: Vec<ProjectionItem> = Vec::new();
            for pi in &r.items {
                let expr = self.lower_expr(&pi.expression);
                let alias_name = pi
                    .alias
                    .as_ref()
                    .map(|v| v.name.name.clone())
                    .unwrap_or_else(|| self.infer_alias_name(&pi.expression));
                let alias = self.scope_stack.bind(
                    &mut self.arenas,
                    &alias_name,
                    BindingKind::ReturnAlias,
                    expr_span(&pi.expression),
                );
                items.push(ProjectionItem {
                    expression: expr,
                    alias,
                });
            }

            // Check for aggregates in RETURN
            let has_aggregate = r.items.iter().any(|pi| self.has_aggregate(&pi.expression));
            if has_aggregate {
                let mut grouping_keys = Vec::new();
                let mut aggregates = Vec::new();
                for pi in &r.items {
                    if self.has_aggregate(&pi.expression) {
                        let (func_id, args, distinct) = match &pi.expression {
                            Expression::FunctionCall(fc) => {
                                let name =
                                    fc.name.last().map(|s| s.name.clone()).unwrap_or_default();
                                let fid = self.arenas.functions.intern(&name, Id);
                                let a = fc.arguments.iter().map(|a| self.lower_expr(a)).collect();
                                (fid, a, fc.distinct)
                            }
                            Expression::CountStar { .. } => {
                                let fid = self.arenas.functions.intern("COUNT", Id);
                                (fid, vec![], false)
                            }
                            _ => {
                                let fid = self.arenas.functions.intern("UNKNOWN", Id);
                                (fid, vec![], false)
                            }
                        };
                        let alias_name = pi
                            .alias
                            .as_ref()
                            .map(|v| v.name.name.clone())
                            .unwrap_or_else(|| self.infer_alias_name(&pi.expression));
                        let alias = self.scope_stack.bind(
                            &mut self.arenas,
                            &alias_name,
                            BindingKind::ReturnAlias,
                            expr_span(&pi.expression),
                        );
                        aggregates.push(AggregateItem {
                            function: func_id,
                            args,
                            distinct,
                            alias,
                        });
                    } else {
                        let expr = self.lower_expr(&pi.expression);
                        let alias_name = pi
                            .alias
                            .as_ref()
                            .map(|v| v.name.name.clone())
                            .unwrap_or_else(|| self.infer_alias_name(&pi.expression));
                        let alias = self.scope_stack.bind(
                            &mut self.arenas,
                            &alias_name,
                            BindingKind::ReturnAlias,
                            expr_span(&pi.expression),
                        );
                        grouping_keys.push(ProjectionItem {
                            expression: expr,
                            alias,
                        });
                    }
                }
                self.current_part_ops
                    .push(Operation::Aggregate(AggregateOp {
                        grouping_keys,
                        aggregates,
                    }));
            } else {
                self.current_part_ops.push(Operation::Project(ProjectOp {
                    items,
                    distinct: r.distinct,
                }));
            }
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
        let name = proc
            .name
            .name
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>()
            .join(".");
        let procedure = self.arenas.functions.intern(&name, Id);
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
            Expression::Literal(lit) => ExprKind::Literal(self.lower_literal(lit)),
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
                let base_id = self.lower_expr(base);
                let label_ids: Vec<LabelId> = labels
                    .iter()
                    .map(|l| self.lower_label_expression_id(l))
                    .collect();
                ExprKind::NodeLabels {
                    base: base_id,
                    labels: label_ids,
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
            Expression::FunctionCall(fc) => {
                let name = fc.name.last().map(|s| s.name.clone()).unwrap_or_default();
                let func_id = self.arenas.functions.intern(&name, Id);
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
                let _scope = self.scope_stack.push_scope(&mut self.arenas);
                let var = self.scope_stack.bind(
                    &mut self.arenas,
                    &lc.variable.name.name,
                    BindingKind::ComprehensionVar,
                    lc.variable.name.span,
                );
                let collection = self.lower_expr(&Expression::Variable(lc.variable.clone()));
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
                let _scope = self.scope_stack.push_scope(&mut self.arenas);
                let var = self.scope_stack.bind(
                    &mut self.arenas,
                    &fe.variable.name.name,
                    BindingKind::ComprehensionVar,
                    fe.variable.name.span,
                );
                let collection = self.lower_expr(&fe.collection);
                let predicate = fe.predicate.as_ref().map(|p| self.lower_expr(p));
                self.scope_stack.pop_scope();
                ExprKind::CollectionFilter {
                    quantifier,
                    variable: var,
                    collection,
                    predicate,
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
            Literal::List(l) => {
                let _elements: Vec<ExprId> =
                    l.elements.iter().map(|e| self.lower_expr(e)).collect();
                // We represent list literals as ExprKind::List, but HirLiteral only has scalar kinds.
                // Build it as a list expression instead.
                HirLiteral::Null // placeholder - the caller should handle lists specially
            }
            Literal::Map(_m) => {
                HirLiteral::Null // placeholder
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
                // Convert pattern+where to a minimal query
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
                let scope = self.scope_stack.push_scope(&mut self.arenas);
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
                let imported: Vec<BindingId> = self
                    .scope_stack
                    .bindings_by_name
                    .iter()
                    .rev()
                    .flat_map(|m| m.values().copied())
                    .collect();
                let query_ast = Query {
                    statements: vec![QueryBody::Regular((**rq).clone())],
                    span: Span::new(0, 0),
                };
                let query = lower(&query_ast).unwrap_or_else(|_| HirQuery {
                    arenas: HirArenas::new(),
                    parts: Vec::new(),
                    diagnostics: Vec::new(),
                });
                ExistsSubquery {
                    query: Box::new(query),
                    imported_bindings: imported,
                }
            }
        }
    }

    fn lower_count_subquery(&mut self, query: &RegularQuery) -> CountSubquery {
        let imported: Vec<BindingId> = self
            .scope_stack
            .bindings_by_name
            .iter()
            .rev()
            .flat_map(|m| m.values().copied())
            .collect();
        let query_ast = Query {
            statements: vec![QueryBody::Regular(query.clone())],
            span: Span::new(0, 0),
        };
        let q = lower(&query_ast).unwrap_or_else(|_| HirQuery {
            arenas: HirArenas::new(),
            parts: Vec::new(),
            diagnostics: Vec::new(),
        });
        CountSubquery {
            query: Box::new(q),
            imported_bindings: imported,
        }
    }

    fn lower_collect_subquery(&mut self, query: &RegularQuery) -> CollectSubquery {
        let imported: Vec<BindingId> = self
            .scope_stack
            .bindings_by_name
            .iter()
            .rev()
            .flat_map(|m| m.values().copied())
            .collect();
        let query_ast = Query {
            statements: vec![QueryBody::Regular(query.clone())],
            span: Span::new(0, 0),
        };
        let q = lower(&query_ast).unwrap_or_else(|_| HirQuery {
            arenas: HirArenas::new(),
            parts: Vec::new(),
            diagnostics: Vec::new(),
        });
        CollectSubquery {
            query: Box::new(q),
            imported_bindings: imported,
        }
    }

    // ── Pattern lowering ─────────────────────────────────────────────────

    fn lower_pattern(&mut self, pattern: &Pattern) -> GraphPattern {
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();
        let mut path_bindings = Vec::new();

        for part in &pattern.parts {
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

    fn lower_node_pattern(&mut self, node: &NodePattern) -> HirNodePattern {
        let binding = node.variable.as_ref().map(|v| {
            self.scope_stack.bind(
                &mut self.arenas,
                &v.name.name,
                BindingKind::PatternBound,
                v.name.span,
            )
        });
        let labels: Vec<LabelId> = node
            .labels
            .iter()
            .map(|l| self.lower_label_expression_id(l))
            .collect();
        let properties = node.properties.as_ref().map(|p| self.lower_properties(p));
        HirNodePattern {
            binding,
            labels,
            properties,
        }
    }

    fn lower_relationship_pattern(
        &mut self,
        rel: &RelationshipPattern,
        left: NodeIndex,
        right: NodeIndex,
    ) -> HirRelationshipPattern {
        let (binding, types, length, properties) = if let Some(detail) = &rel.detail {
            let binding = detail.variable.as_ref().map(|v| {
                self.scope_stack.bind(
                    &mut self.arenas,
                    &v.name.name,
                    BindingKind::PatternBound,
                    v.name.span,
                )
            });
            let types: Vec<RelTypeId> = detail
                .types
                .as_ref()
                .map(|t| self.lower_label_expression_to_rel_types(t))
                .unwrap_or_default();
            let length = detail
                .range
                .as_ref()
                .map(|r| RelationshipLength::Variable {
                    min: r.start.map(|n| n as u32),
                    max: r.end.map(|n| n as u32),
                })
                .unwrap_or(RelationshipLength::Single);
            let properties = detail.properties.as_ref().map(|p| self.lower_properties(p));
            (binding, types, length, properties)
        } else {
            (None, Vec::new(), RelationshipLength::Single, None)
        };

        let direction = match rel.direction {
            RelationshipDirection::Left => HirRelationshipDirection::RightToLeft,
            RelationshipDirection::Right => HirRelationshipDirection::LeftToRight,
            RelationshipDirection::Both => HirRelationshipDirection::Both,
            RelationshipDirection::Undirected => HirRelationshipDirection::Undirected,
        };

        HirRelationshipPattern {
            binding,
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
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();

        let (ns, rs) = self.lower_pattern_element(&part.anonymous.element);
        nodes.extend(ns);
        relationships.extend(rs);

        GraphPattern {
            nodes,
            relationships,
            path_bindings: Vec::new(),
        }
    }

    fn lower_label_expression_id(&mut self, expr: &LabelExpression) -> LabelId {
        match expr {
            LabelExpression::Static(sym) => self.arenas.labels.intern(&sym.name, Id),
            LabelExpression::Dynamic { span, .. }
            | LabelExpression::Or { span, .. }
            | LabelExpression::And { span, .. }
            | LabelExpression::Not { span, .. }
            | LabelExpression::Group { span, .. } => {
                self.diagnostics.push(HirDiagnostic::UnsupportedFeature {
                    feature: "dynamic label expression".to_string(),
                    span: *span,
                });
                Id(usize::MAX)
            }
        }
    }

    fn lower_label_expression_to_rel_types(&mut self, expr: &LabelExpression) -> Vec<RelTypeId> {
        match expr {
            LabelExpression::Static(sym) => {
                vec![self.arenas.relationship_types.intern(&sym.name, Id)]
            }
            LabelExpression::Or { lhs, rhs, .. } => {
                let mut types = self.lower_label_expression_to_rel_types(lhs);
                types.extend(self.lower_label_expression_to_rel_types(rhs));
                types
            }
            LabelExpression::Dynamic { span, .. }
            | LabelExpression::And { span, .. }
            | LabelExpression::Not { span, .. }
            | LabelExpression::Group { span, .. } => {
                self.diagnostics.push(HirDiagnostic::UnsupportedFeature {
                    feature: "dynamic relationship type expression".to_string(),
                    span: *span,
                });
                Vec::new()
            }
        }
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

    fn resolve_or_bind_variable(&mut self, var: &Variable) -> BindingId {
        match self.scope_stack.resolve(&var.name.name) {
            Some(id) => id,
            None => self.scope_stack.bind(
                &mut self.arenas,
                &var.name.name,
                BindingKind::Value,
                var.name.span,
            ),
        }
    }

    fn resolve_or_create_binding(&mut self, var: &Variable) -> BindingId {
        self.resolve_or_bind_variable(var)
    }

    fn infer_alias_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Variable(v) => v.name.name.clone(),
            Expression::PropertyLookup { property, .. } => property.name.name.clone(),
            Expression::FunctionCall(fc) => fc
                .name
                .last()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "expr".to_string()),
            _ => "expr".to_string(),
        }
    }

    fn has_aggregate(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FunctionCall(fc) => {
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
            Expression::BinaryOp { lhs, rhs, .. } => {
                self.has_aggregate(lhs) || self.has_aggregate(rhs)
            }
            Expression::UnaryOp { operand, .. } => self.has_aggregate(operand),
            Expression::Comparison { lhs, operators, .. } => {
                self.has_aggregate(lhs) || operators.iter().any(|(_, rhs)| self.has_aggregate(rhs))
            }
            Expression::Case(case) => {
                case.alternatives
                    .iter()
                    .any(|alt| self.has_aggregate(&alt.when) || self.has_aggregate(&alt.then))
                    || case
                        .scrutinee
                        .as_ref()
                        .is_some_and(|s| self.has_aggregate(s))
                    || case.default.as_ref().is_some_and(|d| self.has_aggregate(d))
            }
            Expression::Parenthesized(inner) => self.has_aggregate(inner),
            _ => false,
        }
    }
}

fn expr_span(expr: &Expression) -> Span {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Number(n) => match n {
                NumberLiteral::Integer(_) => Span::new(0, 0),
                NumberLiteral::Float(_) => Span::new(0, 0),
            },
            Literal::String(s) => s.span,
            Literal::Boolean(_) => Span::new(0, 0),
            Literal::Null => Span::new(0, 0),
            Literal::List(l) => l.span,
            Literal::Map(m) => m.span,
        },
        Expression::Variable(v) => v.name.span,
        Expression::Parameter(p) => p.span,
        Expression::PropertyLookup { span, .. } => *span,
        Expression::NodeLabels { span, .. } => *span,
        Expression::BinaryOp { span, .. } => *span,
        Expression::UnaryOp { span, .. } => *span,
        Expression::Comparison { span, .. } => *span,
        Expression::ListIndex { span, .. } => *span,
        Expression::ListSlice { span, .. } => *span,
        Expression::In { span, .. } => *span,
        Expression::IsNull { span, .. } => *span,
        Expression::FunctionCall(fc) => fc.span,
        Expression::CountStar { span } => *span,
        Expression::Case(case) => case.span,
        Expression::ListComprehension(lc) => lc.span,
        Expression::PatternComprehension(pc) => pc.span,
        Expression::All(fe)
        | Expression::Any(fe)
        | Expression::None(fe)
        | Expression::Single(fe) => fe.span,
        Expression::Parenthesized(_) => Span::new(0, 0),
        Expression::Pattern(rp) => rp.span,
        Expression::Exists(ex) => ex.span,
        Expression::CountSubquery(c) => c.span,
        Expression::CollectSubquery(c) => c.span,
        Expression::MapProjection(mp) => mp.span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_simple_match_return() {
        let query = parse("MATCH (p:Person) RETURN p.name").unwrap();
        let hir = lower(&query).unwrap();
        assert_eq!(hir.parts.len(), 1);
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
        assert!(matches!(&ops[2], Operation::Return(_)));
    }

    #[test]
    fn test_with_aggregation() {
        let query = parse("MATCH (p:Person) WITH count(*) AS n RETURN n").unwrap();
        let hir = lower(&query).unwrap();
        // WITH creates a new query part, so we get 2 parts
        assert_eq!(hir.parts.len(), 2);
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Aggregate(_)));
    }

    #[test]
    fn test_multi_part_query() {
        let query = parse("MATCH (p:Person) WITH p MATCH (p)-[:KNOWS]->(f) RETURN f.name").unwrap();
        let hir = lower(&query).unwrap();
        assert_eq!(hir.parts.len(), 2);
    }

    #[test]
    fn test_unknown_variable_error() {
        let query = parse("MATCH (p:Person) RETURN x.name").unwrap();
        let result = lower(&query);
        assert!(result.is_err());
    }

    #[test]
    fn test_where_after_match() {
        let query = parse("MATCH (p:Person) WHERE p.age > 18 RETURN p.name").unwrap();
        let hir = lower(&query).unwrap();
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
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
        assert!(matches!(&ops[2], Operation::Filter(_)));
    }

    #[test]
    fn test_optional_match() {
        let query = parse("OPTIONAL MATCH (p:Person) RETURN p.name").unwrap();
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::OptionalMatch(_)));
    }

    #[test]
    fn test_create_clause() {
        let query = parse("CREATE (p:Person {name: 'Alice'})").unwrap();
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Create(_)));
    }

    #[test]
    fn test_delete_clause() {
        let query = parse("MATCH (p:Person) DELETE p").unwrap();
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Delete(_)));
    }

    #[test]
    fn test_unwind_clause() {
        let query = parse("UNWIND [1, 2, 3] AS x RETURN x").unwrap();
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Unwind(_)));
    }

    #[test]
    fn test_return_star() {
        let query = parse("MATCH (p:Person) RETURN *").unwrap();
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
    }

    #[test]
    fn test_order_by_and_limit() {
        let query = parse("MATCH (p:Person) RETURN p.name ORDER BY p.name ASC LIMIT 10").unwrap();
        let hir = lower(&query).unwrap();
        let ops = &hir.parts[0].operations;
        assert!(matches!(&ops[0], Operation::Match(_)));
        assert!(matches!(&ops[1], Operation::Project(_)));
        assert!(matches!(&ops[2], Operation::Sort(_)));
        assert!(matches!(&ops[3], Operation::Limit(_)));
        assert!(matches!(&ops[4], Operation::Return(_)));
    }
}
