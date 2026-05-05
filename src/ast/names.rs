//! Identifier and name types used throughout the openCypher AST.
//!
//! All name types carry a [`Span`] so that diagnostics can point to the exact
//! source location.

use crate::error::Span;

/// A symbolic identifier, optionally backtick-quoted in the source.
///
/// Covers node/relationship variables, function names, label names, property
/// key names, and any other identifier-shaped token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolicName {
    /// The resolved identifier text (backtick quotes and escape sequences
    /// have already been stripped).
    pub name: String,
    /// Byte-offset span of the token in the original source.
    pub span: Span,
}

/// A variable binding such as `n`, `r`, or `path` in a query.
///
/// Variables are always backed by a [`SymbolicName`] that carries the source
/// location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    /// The underlying symbolic name.
    pub name: SymbolicName,
}

/// A node label name, e.g. `Person` in `(n:Person)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelName {
    /// The underlying symbolic name.
    pub name: SymbolicName,
}

/// A relationship type name, e.g. `KNOWS` in `-[:KNOWS]->`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelTypeName {
    /// The underlying symbolic name.
    pub name: SymbolicName,
}

/// A property key name, e.g. `name` in `n.name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyKeyName {
    /// The underlying symbolic name.
    pub name: SymbolicName,
}
