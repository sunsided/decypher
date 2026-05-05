//! Typed abstract syntax tree (AST) for openCypher queries.
//!
//! This module contains all AST node types produced by the parser and the
//! internal CST→AST builder that converts the lossless rowan CST into these
//! high-level types.
//!
//! # Structure
//!
//! | Sub-module | Contents |
//! |---|---|
//! | [`query`] | Top-level query types (`Query`, `RegularQuery`, `Union`, …) |
//! | [`clause`] | Clause types (`Match`, `Return`, `With`, `Create`, …) |
//! | [`expr`] | Expression types (`Expression`, `Literal`, operators, …) |
//! | [`names`] | Identifiers (`Variable`, `SymbolicName`, `LabelName`, …) |
//! | [`pattern`] | Graph pattern types (`NodePattern`, `RelationshipPattern`, …) |
//! | [`schema`] | Schema command types (`CreateIndex`, `CreateConstraint`, …) |
//! | [`procedure`] | Procedure call types (`StandaloneCall`, `InQueryCall`, …) |
//! | [`mod@print`] | [`ToCypher`] trait for serialising AST nodes back to text |
//! | [`visit`] | [`visit::Visit`] / [`visit::VisitMut`] traits for traversal |

pub(crate) mod build_cst;
pub mod clause;
pub mod expr;
pub mod names;
pub mod pattern;
pub mod print;
pub mod procedure;
pub mod query;
pub mod schema;
pub mod visit;

pub use crate::ast::expr::Expression;
pub use crate::ast::print::ToCypher;
pub use crate::ast::query::{Query, QueryBody, RegularQuery, SingleQuery, Union};
