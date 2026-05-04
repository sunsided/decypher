use crate::error::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolicName {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: SymbolicName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelName {
    pub name: SymbolicName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelTypeName {
    pub name: SymbolicName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyKeyName {
    pub name: SymbolicName,
}
