use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::expressions::Expression;
use super::patterns::Pattern;
use super::projection::ProjectionBody;
use super::support::{child, child_token, children, AstChildren};
use super::traits::AstNode;

#[derive(Clone, Debug)]
pub enum Clause {
    Match(MatchClause),
    Return(ReturnClause),
    With(WithClause),
    Unwind(UnwindClause),
    Create(CreateClause),
    Merge(MergeClause),
    Set(SetClause),
    Delete(DeleteClause),
    Remove(RemoveClause),
    Where(WhereClause),
}

impl AstNode for Clause {
    fn can_cast(kind: SyntaxKind) -> bool {
        MatchClause::can_cast(kind)
            || ReturnClause::can_cast(kind)
            || WithClause::can_cast(kind)
            || UnwindClause::can_cast(kind)
            || CreateClause::can_cast(kind)
            || MergeClause::can_cast(kind)
            || SetClause::can_cast(kind)
            || DeleteClause::can_cast(kind)
            || RemoveClause::can_cast(kind)
            || WhereClause::can_cast(kind)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if MatchClause::can_cast(syntax.kind()) {
            return MatchClause::cast(syntax).map(Clause::Match);
        }
        if ReturnClause::can_cast(syntax.kind()) {
            return ReturnClause::cast(syntax).map(Clause::Return);
        }
        if WithClause::can_cast(syntax.kind()) {
            return WithClause::cast(syntax).map(Clause::With);
        }
        if UnwindClause::can_cast(syntax.kind()) {
            return UnwindClause::cast(syntax).map(Clause::Unwind);
        }
        if CreateClause::can_cast(syntax.kind()) {
            return CreateClause::cast(syntax).map(Clause::Create);
        }
        if MergeClause::can_cast(syntax.kind()) {
            return MergeClause::cast(syntax).map(Clause::Merge);
        }
        if SetClause::can_cast(syntax.kind()) {
            return SetClause::cast(syntax).map(Clause::Set);
        }
        if DeleteClause::can_cast(syntax.kind()) {
            return DeleteClause::cast(syntax).map(Clause::Delete);
        }
        if RemoveClause::can_cast(syntax.kind()) {
            return RemoveClause::cast(syntax).map(Clause::Remove);
        }
        if WhereClause::can_cast(syntax.kind()) {
            return WhereClause::cast(syntax).map(Clause::Where);
        }
        None
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Clause::Match(it) => it.syntax(),
            Clause::Return(it) => it.syntax(),
            Clause::With(it) => it.syntax(),
            Clause::Unwind(it) => it.syntax(),
            Clause::Create(it) => it.syntax(),
            Clause::Merge(it) => it.syntax(),
            Clause::Set(it) => it.syntax(),
            Clause::Delete(it) => it.syntax(),
            Clause::Remove(it) => it.syntax(),
            Clause::Where(it) => it.syntax(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchClause(SyntaxNode);

impl AstNode for MatchClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MATCH_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MatchClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MatchClause {
    pub fn optional_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_OPTIONAL)
    }

    pub fn pattern(&self) -> Option<Pattern> {
        child(&self.0)
    }

    pub fn where_clause(&self) -> Option<WhereClause> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ReturnClause(SyntaxNode);

impl AstNode for ReturnClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::RETURN_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ReturnClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ReturnClause {
    pub fn projection_body(&self) -> Option<ProjectionBody> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct WithClause(SyntaxNode);

impl AstNode for WithClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::WITH_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(WithClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl WithClause {
    pub fn projection_body(&self) -> Option<ProjectionBody> {
        child(&self.0)
    }

    pub fn where_clause(&self) -> Option<WhereClause> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct UnwindClause(SyntaxNode);

impl AstNode for UnwindClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::UNWIND_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(UnwindClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl UnwindClause {
    pub fn expr(&self) -> Option<Expression> {
        child(&self.0)
    }

    pub fn as_name(&self) -> Option<super::top_level::Variable> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct CreateClause(SyntaxNode);

impl AstNode for CreateClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CREATE_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(CreateClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CreateClause {
    pub fn pattern(&self) -> Option<Pattern> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct MergeClause(SyntaxNode);

impl AstNode for MergeClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MERGE_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MergeClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MergeClause {
    pub fn pattern(&self) -> Option<super::patterns::PatternPart> {
        child(&self.0)
    }

    pub fn actions(&self) -> AstChildren<MergeAction> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct MergeAction(SyntaxNode);

impl AstNode for MergeAction {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MERGE_ACTION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MergeAction(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MergeAction {
    pub fn on_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_ON)
    }

    pub fn match_or_create_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_MATCH)
            .or_else(|| child_token(&self.0, SyntaxKind::KW_CREATE))
    }

    pub fn set_items(&self) -> AstChildren<SetItem> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct SetClause(SyntaxNode);

impl AstNode for SetClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SET_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SetClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SetClause {
    pub fn items(&self) -> AstChildren<SetItem> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct SetItem(SyntaxNode);

impl AstNode for SetItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SET_ITEM
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SetItem(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SetItem {
    pub fn property_expr(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::VARIABLE
                        | SyntaxKind::PROPERTY_LOOKUP
                        | SyntaxKind::PROPERTY_OR_LABELS_EXPR
                        | SyntaxKind::PROPERTY_EXPRESSION
                )
            })
            .next()
            .and_then(Expression::cast)
    }

    pub fn eq_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::EQ)
    }

    pub fn plus_eq_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::PLUSEQ)
    }

    pub fn value_expr(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| {
                !matches!(
                    n.kind(),
                    SyntaxKind::VARIABLE
                        | SyntaxKind::PROPERTY_LOOKUP
                        | SyntaxKind::PROPERTY_OR_LABELS_EXPR
                        | SyntaxKind::PROPERTY_EXPRESSION
                        | SyntaxKind::NODE_LABELS
                        | SyntaxKind::SYMBOLIC_NAME
                )
            })
            .find_map(Expression::cast)
    }

    pub fn node_labels(&self) -> Option<super::patterns::NodeLabels> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct DeleteClause(SyntaxNode);

impl AstNode for DeleteClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::DELETE_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(DeleteClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl DeleteClause {
    pub fn detach_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_DETACH)
    }

    pub fn exprs(&self) -> impl Iterator<Item = Expression> {
        self.0.children().filter_map(Expression::cast)
    }
}

#[derive(Clone, Debug)]
pub struct RemoveClause(SyntaxNode);

impl AstNode for RemoveClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::REMOVE_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RemoveClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RemoveClause {
    pub fn items(&self) -> AstChildren<RemoveItem> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct RemoveItem(SyntaxNode);

impl AstNode for RemoveItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::REMOVE_ITEM
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RemoveItem(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RemoveItem {
    pub fn property_expr(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::VARIABLE
                        | SyntaxKind::PROPERTY_LOOKUP
                        | SyntaxKind::PROPERTY_EXPRESSION
                )
            })
            .find_map(Expression::cast)
    }

    pub fn node_labels(&self) -> Option<super::patterns::NodeLabels> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct WhereClause(SyntaxNode);

impl AstNode for WhereClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::WHERE_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(WhereClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl WhereClause {
    pub fn expr(&self) -> Option<Expression> {
        child(&self.0)
    }
}
