//! Top-level query types for the Cypher AST.
//!
//! A parsed Cypher source is represented as a [`Query`] containing one or
//! more [`QueryBody`] statements. Each statement can be a data query
//! ([`SingleQuery`], [`RegularQuery`]), a standalone procedure call
//! ([`StandaloneCall`]), a schema command ([`SchemaCommand`]), a `SHOW`
//! command, or a `USE` clause.

use crate::ast::clause::{
    Create, Delete, Finish, Foreach, LoadCsv, Match, Merge, Remove, Return, Set, Unwind, With,
};
use crate::ast::expr::Expression;
use crate::ast::procedure::{InQueryCall, StandaloneCall};
use crate::ast::schema::{SchemaCommand, Show, Use};
use crate::error::Span;

/// The root of a parsed Cypher source: a list of top-level statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// The top-level statements in the source, in order.
    pub statements: Vec<QueryBody>,
    /// Byte-offset span covering the entire source.
    pub span: Span,
}

/// A single top-level statement in a query source.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryBody {
    /// A plain single query (single- or multi-part, no `UNION`).
    SingleQuery(SingleQuery),
    /// A query with one or more `UNION` / `UNION ALL` clauses.
    Regular(RegularQuery),
    /// A top-level `CALL procedure(…)` invocation.
    Standalone(StandaloneCall),
    /// A DDL command (`CREATE INDEX`, `CREATE CONSTRAINT`, …).
    SchemaCommand(SchemaCommand),
    /// A `SHOW` command.
    Show(Show),
    /// A `USE` graph-selector clause.
    Use(Use),
}

/// A data query that is either a single-part or multi-part query.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleQuery {
    /// Whether this is a single-part or multi-part (WITH-joined) query.
    pub kind: SingleQueryKind,
}

/// Discriminates single-part from multi-part queries.
#[derive(Debug, Clone, PartialEq)]
pub enum SingleQueryKind {
    /// A query with no `WITH` clause separating reading from returning.
    SinglePart(SinglePartQuery),
    /// A query with one or more intermediate `WITH` clauses.
    MultiPart(MultiPartQuery),
}

/// A query part that reads, optionally updates, and then returns or finishes.
#[derive(Debug, Clone, PartialEq)]
pub struct SinglePartQuery {
    /// Zero or more reading clauses (`MATCH`, `UNWIND`, `CALL`, `LOAD CSV`).
    pub reading_clauses: Vec<ReadingClause>,
    /// The mandatory terminating body: `RETURN`, updating clauses, or `FINISH`.
    pub body: SinglePartBody,
}

/// The body that terminates a [`SinglePartQuery`].
#[derive(Debug, Clone, PartialEq)]
pub enum SinglePartBody {
    /// Ends with a `RETURN` clause.
    Return(Return),
    /// Has updating clauses, optionally followed by `RETURN`.
    Updating {
        /// The ordered list of updating clauses.
        updating: Vec<UpdatingClause>,
        /// An optional trailing `RETURN`.
        return_clause: Option<Return>,
    },
    /// Ends with a `FINISH` clause (GQL extension).
    Finish(Finish),
}

/// A multi-part query: a sequence of WITH-separated parts plus a final part.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPartQuery {
    /// Intermediate parts, each ending with a `WITH` clause.
    pub parts: Vec<MultiPartQueryPart>,
    /// The final part, ending with a `RETURN` (or update/finish).
    pub final_part: SinglePartQuery,
}

/// One intermediate part of a multi-part query.
///
/// Consists of reading clauses, optional updating clauses, and the mandatory
/// `WITH` projection that feeds results to the next part.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPartQueryPart {
    /// Reading clauses at the start of this part.
    pub reading_clauses: Vec<ReadingClause>,
    /// Updating clauses (if any) before the `WITH`.
    pub updating_clauses: Vec<UpdatingClause>,
    /// The `WITH` clause that ends this part and begins the next scope.
    pub with: With,
}

/// A reading clause that introduces or filters graph data without writing.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadingClause {
    /// `MATCH` pattern clause.
    Match(Match),
    /// `UNWIND` list-expansion clause.
    Unwind(Unwind),
    /// In-query procedure call (`CALL proc() YIELD …`).
    InQueryCall(InQueryCall),
    /// `CALL { subquery }` clause.
    CallSubquery(Box<CallSubquery>),
    /// `LOAD CSV FROM … AS …` clause.
    LoadCsv(LoadCsv),
}

/// `CALL { subquery } [IN TRANSACTIONS]`
///
/// Executes an inner query in a correlated sub-context. The optional
/// [`InTransactions`] modifier enables batch processing.
#[derive(Debug, Clone, PartialEq)]
pub struct CallSubquery {
    /// The inner query body.
    pub query: RegularQuery,
    /// Optional `IN TRANSACTIONS` modifier.
    pub in_transactions: Option<InTransactions>,
    /// Byte-offset span of the whole `CALL { … }` construct.
    pub span: Span,
}

/// `IN TRANSACTIONS [OF n ROWS] [ON ERROR {CONTINUE|BREAK|FAIL}]`
///
/// Controls how `CALL { … }` batches its inner query for large writes.
#[derive(Debug, Clone, PartialEq)]
pub struct InTransactions {
    /// Optional batch size expression (`OF n ROWS`).
    pub of_rows: Option<Expression>,
    /// Optional error-handling mode.
    pub on_error: Option<OnErrorBehavior>,
    /// Byte-offset span of the `IN TRANSACTIONS` modifier.
    pub span: Span,
}

/// Error-handling mode for `IN TRANSACTIONS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnErrorBehavior {
    /// Continue processing subsequent batches after an error.
    Continue,
    /// Stop processing but do not roll back already-committed batches.
    Break,
    /// Fail and roll back (default behaviour).
    Fail,
}

/// A write clause that modifies graph data.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdatingClause {
    /// `CREATE` pattern clause.
    Create(Create),
    /// `MERGE` pattern clause.
    Merge(Merge),
    /// `DELETE` clause.
    Delete(Delete),
    /// `SET` clause.
    Set(Set),
    /// `REMOVE` clause.
    Remove(Remove),
    /// `FOREACH` iterating update clause.
    Foreach(Foreach),
}

/// A query with one initial query and zero or more `UNION` branches.
#[derive(Debug, Clone, PartialEq)]
pub struct RegularQuery {
    /// The initial single query.
    pub single_query: SingleQuery,
    /// Subsequent `UNION` / `UNION ALL` branches.
    pub unions: Vec<Union>,
}

/// A single `UNION` or `UNION ALL` branch in a [`RegularQuery`].
#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    /// `true` if `UNION ALL` (duplicates allowed); `false` if `UNION` (deduplicates).
    pub all: bool,
    /// The query body of this union branch.
    pub single_query: SingleQuery,
    /// Byte-offset span of the `UNION [ALL] …` construct.
    pub span: Span,
}
