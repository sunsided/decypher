use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::clauses::Clause;
use super::support::{child, child_token, children, AstChildren};
use super::traits::AstNode;

#[derive(Clone, Debug)]
pub struct SourceFile(SyntaxNode);

impl AstNode for SourceFile {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SOURCE_FILE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SourceFile(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SourceFile {
    pub fn statements(&self) -> AstChildren<Statement> {
        children(&self.0)
    }

    pub fn schema_commands(&self) -> impl Iterator<Item = super::schema::SchemaCommand> {
        self.0
            .children()
            .filter_map(super::schema::SchemaCommand::cast)
    }
}

#[derive(Clone, Debug)]
pub struct Statement(SyntaxNode);

impl AstNode for Statement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::STATEMENT
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Statement(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Statement {
    pub fn clauses(&self) -> AstChildren<Clause> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Variable(SyntaxNode);

impl AstNode for Variable {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::VARIABLE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Variable(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Variable {
    pub fn name(&self) -> Option<SymbolicName> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct SymbolicName(SyntaxNode);

impl AstNode for SymbolicName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SYMBOLIC_NAME
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SymbolicName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SymbolicName {
    pub fn ident_token(&self) -> Option<super::tokens::Ident> {
        use super::traits::AstToken;
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find_map(super::tokens::Ident::cast)
    }
}

#[derive(Clone, Debug)]
pub struct Parameter(SyntaxNode);

impl AstNode for Parameter {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PARAMETER
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Parameter(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ============================================================
// Union — UNION [ALL] RegularQuery
// ============================================================

#[derive(Clone, Debug)]
pub struct Union(SyntaxNode);

impl AstNode for Union {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::UNION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Union(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Union {
    pub fn all_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_ALL)
    }

    pub fn clauses(&self) -> impl Iterator<Item = Clause> {
        self.0.children().filter_map(Clause::cast)
    }

    pub fn inner_unions(&self) -> AstChildren<Union> {
        children(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use assert2::check;

    #[test]
    fn test_source_file_statements() {
        let parse = parser::parse("MATCH (n) RETURN n");
        let source = SourceFile::cast(parse.tree).unwrap();
        let stmts: Vec<_> = source.statements().collect();
        check!(stmts.len() == 1);
    }

    #[test]
    fn test_statement_clauses() {
        let parse = parser::parse("MATCH (n) RETURN n");
        let source = SourceFile::cast(parse.tree).unwrap();
        let stmt = source.statements().next().unwrap();
        let clauses: Vec<_> = stmt.clauses().collect();
        check!(clauses.len() == 2);
    }
}
