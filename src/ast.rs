//! Typed abstract syntax tree (AST) for openCypher queries.
//!
//! This module contains all AST node types produced by the parser and the
//! [`build_cst`] constructor that converts the lossless rowan CST into these
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
//! | [`print`] | [`ToCypher`] trait for serialising AST nodes back to text |
//! | [`visit`] | [`Visit`] / [`VisitMut`] traits for read-only and mutable traversal |
//! | [`build_cst`] | Internal CST→AST builder (not part of the public API) |

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
