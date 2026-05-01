use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::expressions::Expression;
use super::support::{child, child_token, children, AstChildren};
use super::traits::AstNode;

#[derive(Clone, Debug)]
pub struct Pattern(SyntaxNode);

impl AstNode for Pattern {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PATTERN
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Pattern(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Pattern {
    pub fn parts(&self) -> AstChildren<PatternPart> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct PatternPart(SyntaxNode);

impl AstNode for PatternPart {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PATTERN_PART
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PatternPart(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternPart {
    pub fn variable(&self) -> Option<super::top_level::Variable> {
        child(&self.0)
    }

    pub fn anonymous_part(&self) -> Option<AnonymousPatternPart> {
        self.0.children().find_map(AnonymousPatternPart::cast)
    }

    pub fn element(&self) -> Option<PatternElement> {
        self.anonymous_part().and_then(|a| a.element())
    }
}

#[derive(Clone, Debug)]
pub struct AnonymousPatternPart(SyntaxNode);

impl AstNode for AnonymousPatternPart {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ANONYMOUS_PATTERN_PART
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(AnonymousPatternPart(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl AnonymousPatternPart {
    pub fn element(&self) -> Option<PatternElement> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct PatternElement(SyntaxNode);

impl AstNode for PatternElement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PATTERN_ELEMENT
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PatternElement(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternElement {
    pub fn node(&self) -> Option<NodePattern> {
        child(&self.0)
    }

    pub fn chains(&self) -> AstChildren<PatternElementChain> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct PatternElementChain(SyntaxNode);

impl AstNode for PatternElementChain {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PATTERN_ELEMENT_CHAIN
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PatternElementChain(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternElementChain {
    pub fn relationship(&self) -> Option<RelationshipPattern> {
        child(&self.0)
    }

    pub fn node(&self) -> Option<NodePattern> {
        self.0.children().filter_map(NodePattern::cast).last()
    }
}

#[derive(Clone, Debug)]
pub struct NodePattern(SyntaxNode);

impl AstNode for NodePattern {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::NODE_PATTERN
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(NodePattern(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NodePattern {
    pub fn variable(&self) -> Option<super::top_level::Variable> {
        child(&self.0)
    }

    pub fn labels(&self) -> AstChildren<NodeLabels> {
        children(&self.0)
    }

    pub fn properties(&self) -> Option<Properties> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct RelationshipPattern(SyntaxNode);

impl AstNode for RelationshipPattern {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::RELATIONSHIP_PATTERN
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RelationshipPattern(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RelationshipPattern {
    pub fn left_arrow(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::ARROW_LEFT)
            .or_else(|| child_token(&self.0, SyntaxKind::LT))
    }

    pub fn right_arrow(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::ARROW_RIGHT)
            .or_else(|| child_token(&self.0, SyntaxKind::GT))
    }

    pub fn detail(&self) -> Option<RelationshipDetail> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct RelationshipDetail(SyntaxNode);

impl AstNode for RelationshipDetail {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::RELATIONSHIP_DETAIL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RelationshipDetail(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RelationshipDetail {
    pub fn variable(&self) -> Option<super::top_level::Variable> {
        child(&self.0)
    }

    pub fn types(&self) -> Option<RelationshipTypes> {
        child(&self.0)
    }

    pub fn range(&self) -> Option<RangeLiteral> {
        child(&self.0)
    }

    pub fn properties(&self) -> Option<Properties> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct NodeLabels(SyntaxNode);

impl AstNode for NodeLabels {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::NODE_LABELS
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(NodeLabels(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NodeLabels {
    pub fn labels(&self) -> AstChildren<NodeLabel> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct NodeLabel(SyntaxNode);

impl AstNode for NodeLabel {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::NODE_LABEL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(NodeLabel(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NodeLabel {
    pub fn name(&self) -> Option<LabelName> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct LabelName(SyntaxNode);

impl AstNode for LabelName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LABEL_NAME
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(LabelName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl LabelName {
    pub fn symbolic_name(&self) -> Option<super::top_level::SymbolicName> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct RelationshipTypes(SyntaxNode);

impl AstNode for RelationshipTypes {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::RELATIONSHIP_TYPES
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RelationshipTypes(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RelationshipTypes {
    pub fn types(&self) -> AstChildren<RelTypeName> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct RelTypeName(SyntaxNode);

impl AstNode for RelTypeName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::REL_TYPE_NAME
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RelTypeName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RelTypeName {
    pub fn symbolic_name(&self) -> Option<super::top_level::SymbolicName> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct RangeLiteral(SyntaxNode);

impl AstNode for RangeLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::RANGE_LITERAL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RangeLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct Properties(SyntaxNode);

impl AstNode for Properties {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROPERTIES
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Properties(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Properties {
    pub fn map_literal(&self) -> Option<super::expressions::MapLiteral> {
        child(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::syntax::ast::traits::AstNode;
    use assert2::check;

    fn find_match(
        source: &super::super::top_level::SourceFile,
    ) -> super::super::clauses::MatchClause {
        for clause in source.statements().next().unwrap().clauses() {
            if let super::super::clauses::Clause::Match(m) = clause {
                return m;
            }
        }
        panic!("no match clause found");
    }

    #[test]
    fn test_pattern_parts() {
        let parse = crate::parser::parse("MATCH (n:Person) RETURN n");
        let source = super::super::top_level::SourceFile::cast(parse.tree.clone()).unwrap();
        let match_clause = find_match(&source);
        let pattern = match_clause.pattern().unwrap();
        let parts: Vec<_> = pattern.parts().collect();
        check!(parts.len() == 1);
    }

    #[test]
    fn test_node_labels() {
        let parse = parser::parse("MATCH (n:Person) RETURN n");
        let source = super::super::top_level::SourceFile::cast(parse.tree.clone()).unwrap();
        let match_clause = find_match(&source);
        let pattern = match_clause.pattern().unwrap();
        let part = pattern.parts().next().unwrap();
        let element = part.element().unwrap();
        let node = element.node().unwrap();
        let labels: Vec<_> = node.labels().collect();
        check!(labels.len() == 1);
    }
}
