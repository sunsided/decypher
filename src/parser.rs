use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "cypher.pest"]
pub(crate) struct CypherParser;
