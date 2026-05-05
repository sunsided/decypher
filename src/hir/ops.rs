//! HIR pipeline operation types.
//!
//! The HIR decomposes a query into a sequence of [`QueryPart`]s, each of
//! which is a list of [`Operation`]s executed in order. The operations map
//! closely to Cypher clauses but use arena IDs instead of AST nodes.

use super::arena::{BindingId, ExprId, FunctionId, ScopeId};
use super::pattern::GraphPattern;

/// A single pipeline stage between two scope boundaries.
///
/// Corresponds to one "part" of a query (a sequence of clauses between
/// `WITH` boundaries, or the entire query for a single-part query).
#[derive(Debug, Clone)]
pub struct QueryPart {
    /// The scope visible at the start of this part.
    pub input_scope: ScopeId,
    /// The ordered list of operations in this part.
    pub operations: Vec<Operation>,
    /// The scope produced at the end of this part.
    pub output_scope: ScopeId,
}

/// A single pipeline operation.
///
/// Each variant corresponds to one Cypher clause or sub-operation.
#[derive(Debug, Clone)]
pub enum Operation {
    /// `MATCH pattern [WHERE …]`
    Match(MatchOp),
    /// `OPTIONAL MATCH pattern [WHERE …]`
    OptionalMatch(MatchOp),
    /// `UNWIND list AS variable`
    Unwind(UnwindOp),
    /// `WHERE predicate` filter
    Filter(FilterOp),
    /// `WITH` or `RETURN` projection (non-aggregate)
    Project(ProjectOp),
    /// `WITH` or `RETURN` projection with aggregation
    Aggregate(AggregateOp),
    /// `ORDER BY`
    Sort(SortOp),
    /// `SKIP n`
    Skip(SkipOp),
    /// `LIMIT n`
    Limit(LimitOp),
    /// `CREATE pattern`
    Create(CreateOp),
    /// `MERGE pattern [ON MATCH …] [ON CREATE …]`
    Merge(MergeOp),
    /// `SET items`
    Set(SetOp),
    /// `REMOVE items`
    Remove(RemoveOp),
    /// `[DETACH] DELETE targets`
    Delete(DeleteOp),
    /// `CALL { subquery } [IN TRANSACTIONS …]`
    CallSubquery(CallSubqueryOp),
    /// `CALL procedure() YIELD …`
    CallProcedure(CallProcedureOp),
    /// `LOAD CSV FROM … AS variable`
    LoadCsv(LoadCsvOp),
    /// `FOREACH (var IN list | updates)`
    Foreach(ForeachOp),
    /// `UNION [ALL]`
    Union(UnionOp),
    /// `RETURN items`
    Return(ReturnOp),
    /// `FINISH`
    Finish,
}

/// `MATCH` / `OPTIONAL MATCH` operation.
#[derive(Debug, Clone)]
pub struct MatchOp {
    /// The graph pattern to match.
    pub pattern: GraphPattern,
    /// Additional predicate expressions (from `WHERE`).
    pub predicates: Vec<ExprId>,
}

/// `UNWIND list AS variable` operation.
#[derive(Debug, Clone)]
pub struct UnwindOp {
    /// The list expression to expand.
    pub expression: ExprId,
    /// The binding that receives each element.
    pub variable: BindingId,
}

/// A `WHERE` filter predicate operation.
#[derive(Debug, Clone)]
pub struct FilterOp {
    /// The predicate expression.
    pub predicate: ExprId,
}

/// Non-aggregate projection operation (`WITH` / `RETURN`).
#[derive(Debug, Clone)]
pub struct ProjectOp {
    /// The projected items.
    pub items: Vec<ProjectionItem>,
    /// `true` when `WITH DISTINCT` or `RETURN DISTINCT`.
    pub distinct: bool,
}

/// A single projected item: an expression with an alias binding.
#[derive(Debug, Clone)]
pub struct ProjectionItem {
    /// The expression to project.
    pub expression: ExprId,
    /// The binding that receives the result.
    pub alias: BindingId,
}

/// Aggregating projection operation.
#[derive(Debug, Clone)]
pub struct AggregateOp {
    /// Non-aggregate grouping key items.
    pub grouping_keys: Vec<ProjectionItem>,
    /// Aggregate function applications.
    pub aggregates: Vec<AggregateItem>,
}

/// A single aggregate application in an [`AggregateOp`].
#[derive(Debug, Clone)]
pub struct AggregateItem {
    /// The aggregate function.
    pub function: FunctionId,
    /// The argument expressions.
    pub args: Vec<ExprId>,
    /// `true` when called as `f(DISTINCT …)`.
    pub distinct: bool,
    /// The binding that receives the aggregate result.
    pub alias: BindingId,
}

/// `ORDER BY` operation.
#[derive(Debug, Clone)]
pub struct SortOp {
    /// The sort criteria, in order.
    pub items: Vec<SortItem>,
}

/// A single `ORDER BY` criterion.
#[derive(Debug, Clone)]
pub struct SortItem {
    /// The expression to sort by.
    pub expression: ExprId,
    /// The sort direction.
    pub direction: SortDirection,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// `ASC` / `ASCENDING`
    Ascending,
    /// `DESC` / `DESCENDING`
    Descending,
}

/// `SKIP n` operation.
#[derive(Debug, Clone)]
pub struct SkipOp {
    /// The number of rows to skip.
    pub count: ExprId,
}

/// `LIMIT n` operation.
#[derive(Debug, Clone)]
pub struct LimitOp {
    /// The maximum number of rows to return.
    pub count: ExprId,
}

/// `CREATE pattern` operation.
#[derive(Debug, Clone)]
pub struct CreateOp {
    /// The graph pattern describing what to create.
    pub pattern: GraphPattern,
}

/// `MERGE pattern [ON CREATE SET …] [ON MATCH SET …]` operation.
#[derive(Debug, Clone)]
pub struct MergeOp {
    /// The graph pattern to merge.
    pub pattern: GraphPattern,
    /// Set items to apply when the merge creates new data.
    pub on_create: Vec<SetItem>,
    /// Set items to apply when the merge matches existing data.
    pub on_match: Vec<SetItem>,
}

/// `SET items` operation.
#[derive(Debug, Clone)]
pub struct SetOp {
    /// The set items.
    pub items: Vec<SetItem>,
}

/// A single item in a `SET` or `MERGE ON …` operation.
#[derive(Debug, Clone)]
pub enum SetItem {
    /// `property = value`
    SetProperty {
        /// The property expression (e.g. `n.name`).
        target: ExprId,
        /// The new value.
        value: ExprId,
    },
    /// `variable = value` (replace all properties)
    SetVariable {
        /// The node/relationship binding.
        target: BindingId,
        /// The new property map.
        value: ExprId,
    },
    /// `variable:Label1:Label2` — add labels.
    SetLabels {
        /// The node binding.
        node: BindingId,
        /// The labels to add.
        labels: Vec<super::arena::LabelId>,
    },
    /// `variable = map` (replace all properties with `=`)
    ReplaceProperties {
        /// The entity binding.
        entity: BindingId,
        /// The new property map.
        value: ExprId,
    },
    /// `variable += map` (merge properties)
    MergeProperties {
        /// The entity binding.
        entity: BindingId,
        /// The properties to merge in.
        value: ExprId,
    },
}

/// `REMOVE items` operation.
#[derive(Debug, Clone)]
pub struct RemoveOp {
    /// The items to remove.
    pub items: Vec<RemoveItem>,
}

/// A single item in a `REMOVE` operation.
#[derive(Debug, Clone)]
pub enum RemoveItem {
    /// Remove labels from a node.
    Labels {
        /// The node binding.
        node: BindingId,
        /// The labels to remove.
        labels: Vec<super::arena::LabelId>,
    },
    /// Remove a property.
    Property {
        /// The property expression (e.g. `n.name`).
        target: ExprId,
    },
}

/// `[DETACH] DELETE targets` operation.
#[derive(Debug, Clone)]
pub struct DeleteOp {
    /// `true` for `DETACH DELETE`.
    pub detach: bool,
    /// The expressions identifying nodes/relationships to delete.
    pub targets: Vec<ExprId>,
}

/// `CALL { subquery } [IN TRANSACTIONS …]` operation.
#[derive(Debug, Clone)]
pub struct CallSubqueryOp {
    /// Bindings from the outer scope imported into the subquery.
    pub imported_bindings: Vec<BindingId>,
    /// The inner subquery.
    pub query: Box<super::HirQuery>,
    /// Optional `IN TRANSACTIONS` modifier.
    pub in_transactions: Option<InTransactions>,
}

/// `IN TRANSACTIONS [OF n ROWS] [ON ERROR …]` modifier.
#[derive(Debug, Clone)]
pub struct InTransactions {
    /// Optional batch size (`OF n ROWS`).
    pub of_rows: Option<ExprId>,
    /// Error-handling behaviour.
    pub on_error: OnErrorBehavior,
}

/// Error-handling mode for `IN TRANSACTIONS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnErrorBehavior {
    /// Continue to the next batch after an error.
    Continue,
    /// Stop but keep already-committed batches.
    Break,
    /// Fail and roll back.
    Fail,
}

/// `CALL procedure() YIELD …` operation.
#[derive(Debug, Clone)]
pub struct CallProcedureOp {
    /// The interned procedure name.
    pub procedure: FunctionId,
    /// The positional argument expressions.
    pub args: Vec<ExprId>,
    /// The yielded result columns.
    pub yields: Vec<ProcedureYield>,
}

/// A single yielded column from a procedure call.
#[derive(Debug, Clone)]
pub struct ProcedureYield {
    /// The procedure result field name.
    pub field: String,
    /// The binding that receives the field value.
    pub alias: BindingId,
}

/// `LOAD CSV FROM source AS variable` operation.
#[derive(Debug, Clone)]
pub struct LoadCsvOp {
    /// The source URL expression.
    pub source: ExprId,
    /// The binding that receives each CSV row.
    pub variable: BindingId,
    /// `true` when `WITH HEADERS`.
    pub with_headers: bool,
}

/// `FOREACH (variable IN list | operations)` operation.
#[derive(Debug, Clone)]
pub struct ForeachOp {
    /// The loop variable binding.
    pub variable: BindingId,
    /// The list expression to iterate.
    pub list: ExprId,
    /// The updating operations applied for each element.
    pub operations: Vec<Operation>,
}

/// `UNION [ALL]` operation.
#[derive(Debug, Clone)]
pub struct UnionOp {
    /// The branches to union.
    pub branches: Vec<super::HirQuery>,
    /// `false` for `UNION` (deduplicate); `true` for `UNION ALL`.
    pub distinct: bool,
}

/// `RETURN items` finalising operation.
#[derive(Debug, Clone)]
pub struct ReturnOp {
    /// The projected return items.
    pub items: Vec<ProjectionItem>,
    /// `true` when `RETURN DISTINCT`.
    pub distinct: bool,
}
