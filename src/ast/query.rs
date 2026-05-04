use crate::ast::clause::{
    Create, Delete, Finish, Foreach, LoadCsv, Match, Merge, Remove, Return, Set, Unwind, With,
};
use crate::ast::expr::Expression;
use crate::ast::procedure::{InQueryCall, StandaloneCall};
use crate::ast::schema::{SchemaCommand, Show, Use};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub statements: Vec<QueryBody>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryBody {
    SingleQuery(SingleQuery),
    Regular(RegularQuery),
    Standalone(StandaloneCall),
    SchemaCommand(SchemaCommand),
    Show(Show),
    Use(Use),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SingleQuery {
    pub kind: SingleQueryKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SingleQueryKind {
    SinglePart(SinglePartQuery),
    MultiPart(MultiPartQuery),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SinglePartQuery {
    pub reading_clauses: Vec<ReadingClause>,
    pub body: SinglePartBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SinglePartBody {
    Return(Return),
    Updating {
        updating: Vec<UpdatingClause>,
        return_clause: Option<Return>,
    },
    Finish(Finish),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiPartQuery {
    pub parts: Vec<MultiPartQueryPart>,
    pub final_part: SinglePartQuery,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiPartQueryPart {
    pub reading_clauses: Vec<ReadingClause>,
    pub updating_clauses: Vec<UpdatingClause>,
    pub with: With,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadingClause {
    Match(Match),
    Unwind(Unwind),
    InQueryCall(InQueryCall),
    CallSubquery(CallSubquery),
    LoadCsv(LoadCsv),
}

/// CALL { subquery } [IN TRANSACTIONS]
#[derive(Debug, Clone, PartialEq)]
pub struct CallSubquery {
    pub query: RegularQuery,
    pub in_transactions: Option<InTransactions>,
    pub span: Span,
}

/// IN TRANSACTIONS [OF n ROWS] [ON ERROR {CONTINUE|BREAK|FAIL}]
#[derive(Debug, Clone, PartialEq)]
pub struct InTransactions {
    pub of_rows: Option<Expression>,
    pub on_error: Option<OnErrorBehavior>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnErrorBehavior {
    Continue,
    Break,
    Fail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdatingClause {
    Create(Create),
    Merge(Merge),
    Delete(Delete),
    Set(Set),
    Remove(Remove),
    Foreach(Foreach),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegularQuery {
    pub single_query: SingleQuery,
    pub unions: Vec<Union>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    pub all: bool,
    pub single_query: SingleQuery,
    pub span: Span,
}
