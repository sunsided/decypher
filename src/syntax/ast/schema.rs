use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::expressions::MapLiteral;
use super::patterns::NodePattern;
use super::support::{child, child_token, children, AstChildren};
use super::traits::AstNode;

// ============================================================
// SchemaCommand — umbrella enum over schema DDL commands
// ============================================================

#[derive(Clone, Debug)]
pub enum SchemaCommand {
    CreateIndex(CreateIndex),
    DropIndex(DropIndex),
    CreateConstraint(CreateConstraint),
    DropConstraint(DropConstraint),
}

impl AstNode for SchemaCommand {
    fn can_cast(kind: SyntaxKind) -> bool {
        CreateIndex::can_cast(kind)
            || DropIndex::can_cast(kind)
            || CreateConstraint::can_cast(kind)
            || DropConstraint::can_cast(kind)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if CreateIndex::can_cast(syntax.kind()) {
            return CreateIndex::cast(syntax).map(SchemaCommand::CreateIndex);
        }
        if DropIndex::can_cast(syntax.kind()) {
            return DropIndex::cast(syntax).map(SchemaCommand::DropIndex);
        }
        if CreateConstraint::can_cast(syntax.kind()) {
            return CreateConstraint::cast(syntax).map(SchemaCommand::CreateConstraint);
        }
        if DropConstraint::can_cast(syntax.kind()) {
            return DropConstraint::cast(syntax).map(SchemaCommand::DropConstraint);
        }
        None
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            SchemaCommand::CreateIndex(it) => it.syntax(),
            SchemaCommand::DropIndex(it) => it.syntax(),
            SchemaCommand::CreateConstraint(it) => it.syntax(),
            SchemaCommand::DropConstraint(it) => it.syntax(),
        }
    }
}

// ============================================================
// CreateIndex — CREATE [INDEX_KIND] INDEX [IF NOT EXISTS] [name] [FOR pattern] [OPTIONS ...]
// ============================================================

#[derive(Clone, Debug)]
pub struct CreateIndex(SyntaxNode);

impl AstNode for CreateIndex {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CREATE_INDEX
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(CreateIndex(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CreateIndex {
    pub fn if_not_exists(&self) -> bool {
        child_token(&self.0, SyntaxKind::KW_IF).is_some()
    }

    pub fn name(&self) -> Option<SchemaName> {
        child(&self.0)
    }

    pub fn index_kind(&self) -> Option<IndexKind> {
        child(&self.0)
    }

    pub fn label(&self) -> Option<NodePattern> {
        child(&self.0)
    }

    pub fn properties(&self) -> Option<super::patterns::Properties> {
        child(&self.0)
    }

    pub fn options(&self) -> Option<OptionsClause> {
        child(&self.0)
    }
}

// ============================================================
// DropIndex — DROP INDEX [CONCURRENTLY] name [IF EXISTS]
// ============================================================

#[derive(Clone, Debug)]
pub struct DropIndex(SyntaxNode);

impl AstNode for DropIndex {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::DROP_INDEX
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(DropIndex(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl DropIndex {
    pub fn name(&self) -> Option<SchemaName> {
        child(&self.0)
    }

    pub fn if_exists(&self) -> bool {
        child_token(&self.0, SyntaxKind::KW_IF).is_some()
    }

    pub fn concurrently_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_CONCURRENTLY)
    }
}

// ============================================================
// CreateConstraint — CREATE [CONSTRAINT_KIND] CONSTRAINT [name] FOR pattern REQUIRE ...
// ============================================================

#[derive(Clone, Debug)]
pub struct CreateConstraint(SyntaxNode);

impl AstNode for CreateConstraint {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CREATE_CONSTRAINT
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(CreateConstraint(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CreateConstraint {
    pub fn name(&self) -> Option<SchemaName> {
        child(&self.0)
    }

    pub fn constraint_kind(&self) -> Option<ConstraintKind> {
        child(&self.0)
    }

    pub fn options(&self) -> Option<OptionsClause> {
        child(&self.0)
    }
}

// ============================================================
// DropConstraint — DROP CONSTRAINT [CONCURRENTLY] name [IF EXISTS]
// ============================================================

#[derive(Clone, Debug)]
pub struct DropConstraint(SyntaxNode);

impl AstNode for DropConstraint {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::DROP_CONSTRAINT
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(DropConstraint(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl DropConstraint {
    pub fn name(&self) -> Option<SchemaName> {
        child(&self.0)
    }

    pub fn if_exists(&self) -> bool {
        child_token(&self.0, SyntaxKind::KW_IF).is_some()
    }

    pub fn concurrently_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_CONCURRENTLY)
    }
}

// ============================================================
// IndexKind — wraps TEXT/RANGE/POINT/LOOKUP/FULLTEXT keyword
// ============================================================

#[derive(Clone, Debug)]
pub struct IndexKind(SyntaxNode);

impl AstNode for IndexKind {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::INDEX_KIND
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(IndexKind(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ============================================================
// ConstraintKind — wraps IS NULL / IS UNIQUE / IS NODE KEY / PROPERTY TYPE IS (...)
// ============================================================

#[derive(Clone, Debug)]
pub struct ConstraintKind(SyntaxNode);

impl AstNode for ConstraintKind {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CONSTRAINT_KIND
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ConstraintKind(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ConstraintKind {
    pub fn property_types(&self) -> impl Iterator<Item = super::top_level::SymbolicName> {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::SYMBOLIC_NAME)
            .filter_map(super::top_level::SymbolicName::cast)
    }
}

// ============================================================
// OptionsClause — OPTIONS { ... }
// ============================================================

#[derive(Clone, Debug)]
pub struct OptionsClause(SyntaxNode);

impl AstNode for OptionsClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::OPTIONS_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(OptionsClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl OptionsClause {
    pub fn map(&self) -> Option<MapLiteral> {
        child(&self.0)
    }
}

// ============================================================
// SchemaName — named schema element (index/constraint/database name)
// ============================================================

#[derive(Clone, Debug)]
pub struct SchemaName(SyntaxNode);

impl AstNode for SchemaName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SCHEMA_NAME
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SchemaName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SchemaName {
    pub fn symbolic_name(&self) -> Option<super::top_level::SymbolicName> {
        child(&self.0)
    }
}
