//! Rich error types and diagnostics for Cypher parsing.
//!
//! This module provides structured error kinds (`ErrorKind`), human-readable
//! expectations (`Expected`), diagnostic notes (`Note`), and a multi-error
//! wrapper (`Diagnostics`).
//!
//! # Example: matching on error kinds
//! ```ignore
//! use decypher::{parse, ErrorKind, CypherError};
//!
//! match parse("RETURN;") {
//!     Err(CypherError { kind, .. }) => {
//!         match &kind {
//!             ErrorKind::UnexpectedEof { expected } => {
//!                 println!("expected: {:?}", expected);
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(_) => {}
//! }
//! ```

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

/// A span of byte offsets into the original input string.
///
/// `start` and `end` are zero-based byte offsets (not character offsets).
/// The span covers the half-open range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Create a new span covering bytes `[start, end)`.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Compute (line, column) from the span's start position given the source text.
    /// Lines and columns are 1-based.
    pub fn line_col(&self, source: &str) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in source.char_indices() {
            if i >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Extract a snippet of the source around this span, with surrounding context.
    /// Returns `(line_text, start_offset_on_line, end_offset_on_line)`.
    pub fn snippet_line(&self, source: &str) -> (String, usize, usize) {
        let line_start = source[..self.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = source[self.end..]
            .find('\n')
            .map(|i| self.end + i)
            .unwrap_or(source.len());
        let text = source[line_start..line_end].to_string();
        let col_start = self.start - line_start;
        let col_end = self.end - line_start;
        (text, col_start, col_end)
    }
}

/// A value annotated with its source [`Span`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Wrap `value` with the given `span`.
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    /// Transform the wrapped value with `f`, keeping the span unchanged.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }
}

/// A human-readable description of what the parser expected at a given point.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expected {
    /// A keyword like `"MATCH"`, `"RETURN"`, etc.
    Keyword(Cow<'static, str>),
    /// A symbol like `"("`, `")"`, `"+"`, etc.
    Symbol(Cow<'static, str>),
    /// A general category like `"expression"`, `"variable"`, `"literal"`, etc.
    Category(Cow<'static, str>),
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expected::Keyword(kw) => write!(f, "`{}`", kw),
            Expected::Symbol(s) => write!(f, "`{}`", s),
            Expected::Category(c) => write!(f, "{}", c),
        }
    }
}

/// Why a number is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberReason {
    /// The value exceeds the range of the target integer/float type.
    Overflow,
    /// The literal contains a character that is not valid for the given base.
    InvalidDigit,
    /// The literal is empty (e.g. `0x` with no digits following).
    Empty,
    /// Any other reason.
    Other,
}

impl fmt::Display for NumberReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumberReason::Overflow => write!(f, "number too large"),
            NumberReason::InvalidDigit => write!(f, "invalid digit"),
            NumberReason::Empty => write!(f, "empty number"),
            NumberReason::Other => write!(f, "invalid number"),
        }
    }
}

/// The kind of entity a variable holds, as reported by
/// [`ErrorKind::VariableTypeConflict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VariableKind {
    /// A node, bound by `(n)`.
    Node,
    /// A relationship, bound by `-[r]-`.
    Relationship,
    /// A list of relationships, bound by `-[r*]-`.
    Relationships,
    /// A path, bound by `p = …`.
    Path,
    /// A list literal (`WITH [1, 2] AS l`).
    List,
    /// Another literal value (`WITH 1 AS x`).
    Value,
}

impl fmt::Display for VariableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            VariableKind::Node => "a node",
            VariableKind::Relationship => "a relationship",
            VariableKind::Relationships => "a list of relationships",
            VariableKind::Path => "a path",
            VariableKind::List => "a list",
            VariableKind::Value => "a value",
        })
    }
}

/// A set of Cypher value types, as reported by
/// [`ErrorKind::InvalidArgumentType`] and used by
/// [`crate::sema::Signature`].
///
/// Combine types with `|`: `ValueTypes::STRING | ValueTypes::LIST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueTypes(u16);

impl ValueTypes {
    /// `true` / `false`.
    pub const BOOLEAN: Self = Self(1);
    /// 64-bit integers.
    pub const INTEGER: Self = Self(1 << 1);
    /// Floating-point numbers.
    pub const FLOAT: Self = Self(1 << 2);
    /// Strings.
    pub const STRING: Self = Self(1 << 3);
    /// Lists.
    pub const LIST: Self = Self(1 << 4);
    /// Maps.
    pub const MAP: Self = Self(1 << 5);
    /// Nodes.
    pub const NODE: Self = Self(1 << 6);
    /// Relationships.
    pub const RELATIONSHIP: Self = Self(1 << 7);
    /// Paths.
    pub const PATH: Self = Self(1 << 8);
    /// Spatial points.
    pub const POINT: Self = Self(1 << 9);
    /// Dates, times and datetimes.
    pub const TEMPORAL: Self = Self(1 << 10);
    /// Durations.
    pub const DURATION: Self = Self(1 << 11);
    /// Integers and floats.
    pub const NUMBER: Self = Self(Self::INTEGER.0 | Self::FLOAT.0);
    /// Every type; also the type of `null` and of values not known
    /// statically.
    pub const ANY: Self = Self((1 << 12) - 1);

    /// Whether the two sets share a type.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether every type of `other` is in this set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    const NAMES: [&'static str; 12] = [
        "a Boolean",
        "an Integer",
        "a Float",
        "a String",
        "a List",
        "a Map",
        "a Node",
        "a Relationship",
        "a Path",
        "a Point",
        "a temporal value",
        "a Duration",
    ];
}

impl std::ops::BitOr for ValueTypes {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl fmt::Display for ValueTypes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::ANY {
            return f.write_str("any value");
        }
        let names: Vec<_> = (0..Self::NAMES.len())
            .filter(|bit| self.0 & (1 << bit) != 0)
            .map(|bit| Self::NAMES[bit])
            .collect();
        match names.as_slice() {
            [] => f.write_str("no value"),
            [one] => f.write_str(one),
            [rest @ .., last] => write!(f, "{} or {last}", rest.join(", ")),
        }
    }
}

/// The structured kind of a parse/AST error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The parser encountered an unexpected token.
    UnexpectedToken {
        expected: Vec<Expected>,
        found: String,
    },
    /// The input ended before the parser expected it to.
    UnexpectedEof { expected: Vec<Expected> },
    /// A string literal was opened but never closed.
    UnterminatedString,
    /// A comment was opened but never closed.
    UnterminatedComment,
    /// An escape sequence inside a string was invalid.
    InvalidEscape { sequence: String },
    /// A numeric literal was malformed.
    InvalidNumber { raw: String, reason: NumberReason },
    /// The input was empty.
    EmptyInput,
    /// A required clause was missing after another clause.
    MissingClause {
        clause: &'static str,
        after: &'static str,
    },
    /// A clause appeared more than once where only one is allowed.
    DuplicateClause { clause: &'static str },
    /// An assignment was syntactically or semantically invalid.
    InvalidAssignment { reason: &'static str },
    /// A grammar production is not yet supported by the AST builder.
    Unsupported { production: Cow<'static, str> },
    /// An internal error — last-resort fallback.
    Internal { message: String },
    /// A variable was referenced but never bound in scope (semantic).
    UnresolvedVariable { name: String },
    /// A variable was redeclared in the same scope (semantic).
    RedeclaredVariable { name: String, first_span: Span },
    /// WITH/RETURN mixes aggregates and non-grouping expressions (semantic).
    AggregationMix { non_grouping: Vec<String> },
    /// DISTINCT used where it is not allowed (semantic).
    DistinctNotAllowed,
    /// An aggregate function called where aggregation is not allowed, such
    /// as `MATCH … WHERE count(n) > 1` (semantic).
    AggregateNotAllowed { function: String },
    /// `UNION` branches return different columns (semantic).
    UnionColumnMismatch,
    /// Clauses that cannot be combined, such as `UNION` with `UNION ALL`
    /// (semantic).
    InvalidClauseComposition { reason: &'static str },
    /// A relationship pattern its clause cannot use, such as an untyped
    /// relationship in `CREATE` (semantic).
    InvalidRelationshipPattern { reason: &'static str },
    /// A `DELETE` target that is not a node, relationship or path, such as
    /// `DELETE n:Person` (semantic).
    InvalidDeleteTarget,
    /// A `WITH` item other than a variable without `AS` alias (semantic).
    MissingAlias,
    /// `RETURN *` with no variables in scope (semantic).
    NoVariablesInScope,
    /// A `SKIP` or `LIMIT` value that is not a constant, non-negative
    /// integer, such as `SKIP n.count` or `LIMIT -1` (semantic).
    InvalidSkipLimit { reason: &'static str },
    /// A parameter used as the property map of a `MATCH` or `MERGE` pattern,
    /// such as `MATCH (n $param)` (semantic).
    InvalidParameterUse,
    /// A non-deterministic function inside an aggregate, such as
    /// `count(rand())` (semantic).
    NonDeterministicAggregate,
    /// An operand or argument whose type is known statically and is not one
    /// the operator, clause or function accepts, such as `NOT 'a'` or
    /// `labels(p)` on a path (semantic).
    InvalidArgumentType {
        /// What takes the value: `NOT`, `WHERE`, ``argument 1 of `labels()` ``.
        context: Cow<'static, str>,
        /// The types accepted there.
        expected: ValueTypes,
        /// The types the value can have.
        found: ValueTypes,
    },
    /// A call with too few or too many arguments for the function or
    /// procedure's signature (semantic).
    InvalidNumberOfArguments {
        /// The dot-joined function or procedure name.
        name: String,
        /// The fewest arguments accepted.
        min: u16,
        /// The most arguments accepted, or `None` when unbounded.
        max: Option<u16>,
        /// The number of arguments passed.
        found: u16,
    },
    /// A function the [`crate::sema::Catalog`] does not know, reported only
    /// by a strict catalog (semantic).
    UnknownFunction { name: String },
    /// A procedure the [`crate::sema::Catalog`] does not know, reported only
    /// by a strict catalog (semantic).
    ProcedureNotFound { name: String },
    /// A pattern expression used as a value rather than a predicate, such
    /// as `RETURN size((n)-->())` (semantic).
    PatternExpressionOutsidePredicate,
    /// A variable used as two kinds of entity, such as a node and a
    /// relationship in `MATCH (r) MATCH ()-[r]-()` (semantic).
    VariableTypeConflict {
        name: String,
        bound_as: VariableKind,
        used_as: VariableKind,
    },
    /// Invalid reference in ORDER BY / WHERE after WITH (semantic).
    InvalidReference { name: String, reason: &'static str },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::UnexpectedToken { expected, found } => {
                write!(f, "unexpected token `{}`, expected ", found)?;
                fmt_expected_list(expected, f)
            }
            ErrorKind::UnexpectedEof { expected } => {
                write!(f, "unexpected end of input, expected ")?;
                fmt_expected_list(expected, f)
            }
            ErrorKind::UnterminatedString => write!(f, "unterminated string literal"),
            ErrorKind::UnterminatedComment => write!(f, "unterminated comment"),
            ErrorKind::InvalidEscape { sequence } => {
                write!(f, "invalid escape sequence: `{}`", sequence)
            }
            ErrorKind::InvalidNumber { raw, reason } => {
                write!(f, "invalid number `{}`: {}", raw, reason)
            }
            ErrorKind::EmptyInput => write!(f, "empty input"),
            ErrorKind::MissingClause { clause, after } => {
                write!(f, "expected {} after `{}`", clause, after)
            }
            ErrorKind::DuplicateClause { clause } => {
                write!(f, "duplicate `{}` clause", clause)
            }
            ErrorKind::InvalidAssignment { reason } => {
                write!(f, "invalid assignment: {}", reason)
            }
            ErrorKind::Unsupported { production } => {
                write!(f, "unsupported grammar production: {}", production)
            }
            ErrorKind::Internal { message } => {
                write!(f, "internal error: {}", message)
            }
            ErrorKind::UnresolvedVariable { name } => {
                write!(f, "unresolved variable `{}`", name)
            }
            ErrorKind::RedeclaredVariable { name, first_span } => {
                write!(
                    f,
                    "variable `{}` redeclared (first declared at {:?})",
                    name, first_span
                )
            }
            ErrorKind::AggregationMix { non_grouping } => {
                write!(
                    f,
                    "mixing aggregate and non-aggregate expressions: non-grouping keys: {:?}",
                    non_grouping
                )
            }
            ErrorKind::DistinctNotAllowed => {
                write!(f, "DISTINCT is not allowed in this context")
            }
            ErrorKind::AggregateNotAllowed { function } => {
                write!(f, "aggregate function `{function}` is not allowed here")
            }
            ErrorKind::UnionColumnMismatch => {
                write!(f, "all parts of a UNION must return the same columns")
            }
            ErrorKind::InvalidClauseComposition { reason }
            | ErrorKind::InvalidRelationshipPattern { reason }
            | ErrorKind::InvalidSkipLimit { reason } => write!(f, "{reason}"),
            ErrorKind::InvalidParameterUse => write!(
                f,
                "parameter maps cannot be used in MATCH or MERGE patterns; use a literal map such as {{key: $value}}"
            ),
            ErrorKind::InvalidArgumentType {
                context,
                expected,
                found,
            } => write!(f, "{context} expects {expected}, but got {found}"),
            ErrorKind::InvalidNumberOfArguments {
                name,
                min,
                max,
                found,
            } => {
                let takes = match max {
                    Some(max) if max == min => format!("{min}"),
                    Some(max) => format!("{min} to {max}"),
                    None => format!("at least {min}"),
                };
                // "1 argument", "at least 1 argument", "1 to 3 arguments"
                let s = if *min == 1 && max.is_none_or(|max| max == 1) {
                    ""
                } else {
                    "s"
                };
                write!(f, "`{name}` takes {takes} argument{s}, but got {found}")
            }
            ErrorKind::UnknownFunction { name } => write!(f, "unknown function `{name}`"),
            ErrorKind::ProcedureNotFound { name } => write!(f, "unknown procedure `{name}`"),
            ErrorKind::PatternExpressionOutsidePredicate => write!(
                f,
                "a pattern expression can only be used as a predicate; use COUNT {{ … }} to count matches or a pattern comprehension to collect them"
            ),
            ErrorKind::NonDeterministicAggregate => {
                write!(
                    f,
                    "aggregates cannot use the non-deterministic function rand()"
                )
            }
            ErrorKind::VariableTypeConflict {
                name,
                bound_as,
                used_as,
            } => write!(
                f,
                "variable `{name}` is {bound_as} and cannot be used as {used_as}"
            ),
            ErrorKind::InvalidDeleteTarget => {
                write!(f, "DELETE expects nodes, relationships or paths")
            }
            ErrorKind::MissingAlias => {
                write!(f, "expressions in WITH must be aliased (use AS)")
            }
            ErrorKind::NoVariablesInScope => {
                write!(
                    f,
                    "RETURN * is not allowed when there are no variables in scope"
                )
            }
            ErrorKind::InvalidReference { name, reason } => {
                write!(f, "invalid reference to `{}`: {}", name, reason)
            }
        }
    }
}

/// Format a comma-separated list of [`Expected`] items into `f`.
///
/// Produces natural-language output like `nothing in particular`, a single
/// item, or a comma-separated list for multiple items.
fn fmt_expected_list(expected: &[Expected], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if expected.is_empty() {
        write!(f, "nothing in particular")
    } else if expected.len() == 1 {
        write!(f, "{}", expected[0])
    } else {
        for (i, e) in expected.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

/// Severity level for a diagnostic note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteLevel {
    /// Informational — context or background.
    Info,
    /// A non-fatal warning.
    Warning,
    /// A suggested fix or corrective action.
    Help,
}

impl fmt::Display for NoteLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteLevel::Info => write!(f, "info"),
            NoteLevel::Warning => write!(f, "warning"),
            NoteLevel::Help => write!(f, "help"),
        }
    }
}

/// A diagnostic note attached to an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The source span this note points at (may be `Span::new(0,0)` for
    /// non-positional notes).
    pub span: Span,
    /// The human-readable note message.
    pub message: Cow<'static, str>,
    /// The severity / role of this note.
    pub level: NoteLevel,
}

/// The top-level error type returned by the parser.
///
/// Every error carries a structured [`ErrorKind`] discriminant, a byte-offset
/// [`Span`], and optional contextual notes. The source text is stored
/// alongside the error so that [`CypherError::render`] and the [`Display`]
/// impl can produce annotated output without the caller needing to keep the
/// original input alive.
///
/// [`Display`]: std::fmt::Display
#[derive(Debug, Clone)]
pub struct CypherError {
    /// The structured error category.
    pub kind: ErrorKind,
    /// Byte-offset span pointing at the offending token or construct.
    pub span: Span,
    /// Optional label identifying the source file or query.
    pub source_label: Option<Arc<str>>,
    /// Contextual notes (help, warnings, additional pointers).
    pub notes: Vec<Note>,
    /// The original source text, if available for inline rendering.
    pub(crate) source: Option<Arc<str>>,
}

impl CypherError {
    /// Return a shared reference to the structured error kind.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Return the byte-offset span of the error.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Return the notes attached to this error.
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Return the source-file label, if any.
    pub fn source_label(&self) -> Option<&str> {
        self.source_label.as_deref()
    }

    /// Render this error as a human-readable rustc-style diagnostic string.
    ///
    /// `source` must be the original query text that was parsed.
    ///
    /// # Example
    ///
    /// ```
    /// use decypher::parse;
    ///
    /// let err = parse("RETURN;").unwrap_err();
    /// let rendered = err.render("RETURN;");
    /// assert!(rendered.contains("error:"));
    /// ```
    pub fn render(&self, source: &str) -> String {
        render_diagnostic(self, source)
    }
}

impl fmt::Display for CypherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref src) = self.source {
            write!(f, "{}", render_diagnostic(self, src))
        } else {
            write!(f, "error: {}", self.kind)
        }
    }
}

impl std::error::Error for CypherError {}

impl From<std::convert::Infallible> for CypherError {
    fn from(e: std::convert::Infallible) -> Self {
        match e {}
    }
}

/// A collection of diagnostic errors (for multi-error reporting).
///
/// Returned by [`crate::parse_all`] and [`crate::parse_with_options`], and
/// used internally by [`crate::sema`] and the HIR lowering pass for
/// semantic-analysis and HIR-lowering failures.
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// All errors collected during parsing, semantic analysis, or HIR lowering.
    pub errors: Vec<CypherError>,
}

impl Diagnostics {
    /// Return `true` if there are no errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Return the number of errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Iterate over the errors.
    pub fn iter(&self) -> impl Iterator<Item = &CypherError> {
        self.errors.iter()
    }
}

impl IntoIterator for Diagnostics {
    type Item = CypherError;
    type IntoIter = std::vec::IntoIter<CypherError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a Diagnostics {
    type Item = &'a CypherError;
    type IntoIter = std::slice::Iter<'a, CypherError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, err) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            if let Some(ref src) = err.source {
                write!(f, "{}", render_diagnostic(err, src))?;
            } else {
                write!(f, "error: {}", err.kind)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A type alias for `std::result::Result<T, CypherError>`.
pub type Result<T> = std::result::Result<T, CypherError>;

#[cfg(feature = "miette")]
mod miette_impl;
mod render;

pub use render::render_diagnostic;
