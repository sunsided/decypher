use crate::ast::clause::{Create, Delete, Match, Merge, Remove, Return, Set, Unwind, With};
use crate::ast::procedure::{InQueryCall, StandaloneCall};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub statements: Vec<QueryBody>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryBody {
    SingleQuery(SingleQuery),
    Standalone(StandaloneCall),
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdatingClause {
    Create(Create),
    Merge(Merge),
    Delete(Delete),
    Set(Set),
    Remove(Remove),
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

pub enum Statement {
    Query(RegularQuery),
    StandaloneCall(StandaloneCall),
}
