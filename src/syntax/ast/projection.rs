use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::expressions::Expression;
use super::support::{child, child_token, children, AstChildren};
use super::traits::AstNode;

#[derive(Clone, Debug)]
pub struct ProjectionBody(SyntaxNode);

impl AstNode for ProjectionBody {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROJECTION_BODY
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ProjectionBody(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ProjectionBody {
    pub fn distinct_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_DISTINCT)
    }

    pub fn star_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::STAR)
    }

    pub fn items(&self) -> impl Iterator<Item = ProjectionItem> {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::PROJECTION_ITEMS)
            .into_iter()
            .flat_map(|n| n.children())
            .filter_map(ProjectionItem::cast)
    }

    pub fn order_by(&self) -> Option<OrderBy> {
        child(&self.0)
    }

    pub fn skip(&self) -> Option<SkipClause> {
        child(&self.0)
    }

    pub fn limit(&self) -> Option<LimitClause> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ProjectionItems(SyntaxNode);

impl AstNode for ProjectionItems {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROJECTION_ITEMS
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ProjectionItems(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ProjectionItems {
    pub fn items(&self) -> impl Iterator<Item = ProjectionItem> {
        self.0.children().filter_map(ProjectionItem::cast)
    }
}

#[derive(Clone, Debug)]
pub struct ProjectionItem(SyntaxNode);

impl AstNode for ProjectionItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROJECTION_ITEM
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ProjectionItem(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ProjectionItem {
    pub fn expr(&self) -> Option<Expression> {
        // Collect all expression children in order, but stop at KW_AS so the
        // alias VARIABLE after `AS` is not picked up as the projection's expr.
        // The CST stores them flat, e.g. NUMBER_LITERAL, ADD_SUB_EXPR for
        // "1 + 2", so the last expression node before KW_AS is the top-level
        // operator; lhs() on that node will look at its preceding siblings.
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if let Some(tok) = child.as_token() {
                if tok.kind() == SyntaxKind::KW_AS {
                    break;
                }
                continue;
            }
            if let Some(node) = child.as_node() {
                if let Some(e) = Expression::cast(node.clone()) {
                    last = Some(e);
                }
            }
        }
        last
    }

    pub fn as_name(&self) -> Option<super::top_level::Variable> {
        // Look for a VARIABLE that appears after a KW_AS token
        let mut found_as = false;
        for child in self.0.children_with_tokens() {
            if let Some(token) = child.as_token() {
                if token.kind() == SyntaxKind::KW_AS {
                    found_as = true;
                }
            } else if found_as {
                if let Some(node) = child.as_node() {
                    if let Some(v) = super::top_level::Variable::cast(node.clone()) {
                        return Some(v);
                    }
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct OrderBy(SyntaxNode);

impl AstNode for OrderBy {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ORDER_BY
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(OrderBy(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl OrderBy {
    pub fn items(&self) -> AstChildren<SortItem> {
        children(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug)]
pub struct SortItem(SyntaxNode);

impl AstNode for SortItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SORT_ITEM
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SortItem(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SortItem {
    pub fn expr(&self) -> Option<Expression> {
        // In the flat CST, the expression is the LAST expression child.
        // e.g. ORDER BY n.name ASC produces:
        //   SORT_ITEM
        //     └── VARIABLE (n)
        //     └── PROPERTY_LOOKUP (.name)  ← this is the root
        //     └── KW_ASC (token)
        self.0.children().filter_map(Expression::cast).last()
    }

    pub fn direction(&self) -> Option<SortDirection> {
        if child_token(&self.0, SyntaxKind::KW_ASC).is_some()
            || child_token(&self.0, SyntaxKind::KW_ASCENDING).is_some()
        {
            Some(SortDirection::Ascending)
        } else if child_token(&self.0, SyntaxKind::KW_DESC).is_some()
            || child_token(&self.0, SyntaxKind::KW_DESCENDING).is_some()
        {
            Some(SortDirection::Descending)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkipClause(SyntaxNode);

impl AstNode for SkipClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SKIP_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(SkipClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SkipClause {
    pub fn expr(&self) -> Option<Expression> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct LimitClause(SyntaxNode);

impl AstNode for LimitClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LIMIT_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(LimitClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl LimitClause {
    pub fn expr(&self) -> Option<Expression> {
        child(&self.0)
    }
}
