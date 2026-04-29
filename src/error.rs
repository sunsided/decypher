use std::fmt;

/// A span of byte offsets into the original input string.
///
/// `start` and `end` are zero-based byte offsets (not character offsets).
/// The span covers the half-open range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A value annotated with its source [`Span`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CypherError {
    #[error("syntax error: {0}")]
    Syntax(Box<pest::error::Error<crate::parser::Rule>>),

    #[error("AST build error at {span:?}: {message}")]
    Ast { message: String, span: Span },

    #[error("unsupported grammar production: {0}")]
    Unsupported(&'static str),
}

impl From<pest::error::Error<crate::parser::Rule>> for CypherError {
    fn from(err: pest::error::Error<crate::parser::Rule>) -> Self {
        CypherError::Syntax(Box::new(err))
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

pub type Result<T> = std::result::Result<T, CypherError>;
