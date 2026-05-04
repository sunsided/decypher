use super::arena::{BindingId, ExprId, FunctionId, ScopeId};
use super::pattern::GraphPattern;

/// A single query part: a pipeline of operations between two scope boundaries.
#[derive(Debug, Clone)]
pub struct QueryPart {
    pub input_scope: ScopeId,
    pub operations: Vec<Operation>,
    pub output_scope: ScopeId,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Match(MatchOp),
    OptionalMatch(MatchOp),
    Unwind(UnwindOp),
    Filter(FilterOp),
    Project(ProjectOp),
    Aggregate(AggregateOp),
    Sort(SortOp),
    Skip(SkipOp),
    Limit(LimitOp),
    Create(CreateOp),
    Merge(MergeOp),
    Set(SetOp),
    Remove(RemoveOp),
    Delete(DeleteOp),
    CallSubquery(CallSubqueryOp),
    CallProcedure(CallProcedureOp),
    LoadCsv(LoadCsvOp),
    Foreach(ForeachOp),
    Union(UnionOp),
    Return(ReturnOp),
    Finish,
}

#[derive(Debug, Clone)]
pub struct MatchOp {
    pub pattern: GraphPattern,
    pub predicates: Vec<ExprId>,
}

#[derive(Debug, Clone)]
pub struct UnwindOp {
    pub expression: ExprId,
    pub variable: BindingId,
}

#[derive(Debug, Clone)]
pub struct FilterOp {
    pub predicate: ExprId,
}

#[derive(Debug, Clone)]
pub struct ProjectOp {
    pub items: Vec<ProjectionItem>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectionItem {
    pub expression: ExprId,
    pub alias: BindingId,
}

#[derive(Debug, Clone)]
pub struct AggregateOp {
    pub grouping_keys: Vec<ProjectionItem>,
    pub aggregates: Vec<AggregateItem>,
}

#[derive(Debug, Clone)]
pub struct AggregateItem {
    pub function: FunctionId,
    pub args: Vec<ExprId>,
    pub distinct: bool,
    pub alias: BindingId,
}

#[derive(Debug, Clone)]
pub struct SortOp {
    pub items: Vec<SortItem>,
}

#[derive(Debug, Clone)]
pub struct SortItem {
    pub expression: ExprId,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub struct SkipOp {
    pub count: ExprId,
}

#[derive(Debug, Clone)]
pub struct LimitOp {
    pub count: ExprId,
}

#[derive(Debug, Clone)]
pub struct CreateOp {
    pub pattern: GraphPattern,
}

#[derive(Debug, Clone)]
pub struct MergeOp {
    pub pattern: GraphPattern,
    pub on_create: Vec<SetItem>,
    pub on_match: Vec<SetItem>,
}

#[derive(Debug, Clone)]
pub struct SetOp {
    pub items: Vec<SetItem>,
}

#[derive(Debug, Clone)]
pub enum SetItem {
    SetProperty {
        target: ExprId,
        value: ExprId,
    },
    SetVariable {
        target: BindingId,
        value: ExprId,
    },
    SetLabels {
        node: BindingId,
        labels: Vec<super::arena::LabelId>,
    },
    ReplaceProperties {
        entity: BindingId,
        value: ExprId,
    },
    MergeProperties {
        entity: BindingId,
        value: ExprId,
    },
}

#[derive(Debug, Clone)]
pub struct RemoveOp {
    pub items: Vec<RemoveItem>,
}

#[derive(Debug, Clone)]
pub enum RemoveItem {
    Labels {
        node: BindingId,
        labels: Vec<super::arena::LabelId>,
    },
    Property {
        target: ExprId,
    },
}

#[derive(Debug, Clone)]
pub struct DeleteOp {
    pub detach: bool,
    pub targets: Vec<ExprId>,
}

#[derive(Debug, Clone)]
pub struct CallSubqueryOp {
    pub imported_bindings: Vec<BindingId>,
    pub query: Box<super::HirQuery>,
    pub in_transactions: Option<InTransactions>,
}

#[derive(Debug, Clone)]
pub struct InTransactions {
    pub of_rows: Option<ExprId>,
    pub on_error: OnErrorBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnErrorBehavior {
    Continue,
    Break,
    Fail,
}

#[derive(Debug, Clone)]
pub struct CallProcedureOp {
    pub procedure: FunctionId,
    pub args: Vec<ExprId>,
    pub yields: Vec<ProcedureYield>,
}

#[derive(Debug, Clone)]
pub struct ProcedureYield {
    pub field: String,
    pub alias: BindingId,
}

#[derive(Debug, Clone)]
pub struct LoadCsvOp {
    pub source: ExprId,
    pub variable: BindingId,
    pub with_headers: bool,
}

#[derive(Debug, Clone)]
pub struct ForeachOp {
    pub variable: BindingId,
    pub list: ExprId,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone)]
pub struct UnionOp {
    pub branches: Vec<super::HirQuery>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub struct ReturnOp {
    pub items: Vec<ProjectionItem>,
    pub distinct: bool,
}
