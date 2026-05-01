use std::marker::PhantomData;

use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::AstNode;

pub fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
    parent.children().find_map(N::cast)
}

pub fn children<N: AstNode>(parent: &SyntaxNode) -> AstChildren<N> {
    AstChildren {
        inner: parent.children(),
        _marker: PhantomData,
    }
}

pub fn child_token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(|el| el.into_token())
        .find(|t| t.kind() == kind)
}

pub fn child_tokens(parent: &SyntaxNode, kind: SyntaxKind) -> impl Iterator<Item = SyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(move |el| el.into_token())
        .filter(move |t| t.kind() == kind)
}

pub struct AstChildren<N> {
    inner: crate::syntax::SyntaxNodeChildren,
    _marker: PhantomData<N>,
}

impl<N: AstNode> Iterator for AstChildren<N> {
    type Item = N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find_map(N::cast)
    }
}
