use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::support::{AstChildren, child, child_token, children};
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
    pub fn variable(&self) -> Option<super::expressions::Variable> {
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
        matches!(
            kind,
            SyntaxKind::PATTERN_ELEMENT | SyntaxKind::QUANTIFIED_PATH_PATTERN
        )
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

    pub fn quantifier(&self) -> Option<RelationshipQuantifier> {
        child(&self.0)
    }

    pub fn inner(&self) -> Option<PatternElement> {
        self.0.children().find_map(|node| {
            if node.kind() == self.0.kind() {
                None
            } else {
                PatternElement::cast(node)
            }
        })
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
    pub fn variable(&self) -> Option<super::expressions::Variable> {
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

    pub fn quantifier(&self) -> Option<RelationshipQuantifier> {
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
    pub fn variable(&self) -> Option<super::expressions::Variable> {
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

    pub fn expression(&self) -> Option<LabelExpression> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct LabelExpression(SyntaxNode);

impl AstNode for LabelExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LABEL_EXPRESSION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(LabelExpression(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl LabelExpression {
    pub fn root(&self) -> Option<LabelExprNode> {
        self.0.children().find_map(LabelExprNode::cast)
    }
}

#[derive(Clone, Debug)]
pub enum LabelExprNode {
    Or(LabelOr),
    And(LabelAnd),
    Not(LabelNot),
    Paren(LabelParen),
    Atom(LabelAtom),
}

impl AstNode for LabelExprNode {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LABEL_OR
                | SyntaxKind::LABEL_AND
                | SyntaxKind::LABEL_NOT
                | SyntaxKind::LABEL_PAREN
                | SyntaxKind::LABEL_ATOM
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::LABEL_OR => LabelOr::cast(syntax).map(Self::Or),
            SyntaxKind::LABEL_AND => LabelAnd::cast(syntax).map(Self::And),
            SyntaxKind::LABEL_NOT => LabelNot::cast(syntax).map(Self::Not),
            SyntaxKind::LABEL_PAREN => LabelParen::cast(syntax).map(Self::Paren),
            SyntaxKind::LABEL_ATOM => LabelAtom::cast(syntax).map(Self::Atom),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Or(it) => it.syntax(),
            Self::And(it) => it.syntax(),
            Self::Not(it) => it.syntax(),
            Self::Paren(it) => it.syntax(),
            Self::Atom(it) => it.syntax(),
        }
    }
}

macro_rules! label_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                if Self::can_cast(syntax.kind()) {
                    Some(Self(syntax))
                } else {
                    None
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

label_node!(LabelOr, LABEL_OR);
label_node!(LabelAnd, LABEL_AND);
label_node!(LabelNot, LABEL_NOT);
label_node!(LabelParen, LABEL_PAREN);
label_node!(LabelAtom, LABEL_ATOM);
label_node!(DynamicLabel, DYNAMIC_LABEL);
label_node!(DynamicRelType, DYNAMIC_REL_TYPE);
label_node!(RelationshipQuantifier, RELATIONSHIP_QUANTIFIER);

impl LabelOr {
    pub fn items(&self) -> AstChildren<LabelExprNode> {
        children(&self.0)
    }
}

impl LabelAnd {
    pub fn items(&self) -> AstChildren<LabelExprNode> {
        children(&self.0)
    }
}

impl LabelNot {
    pub fn inner(&self) -> Option<LabelExprNode> {
        child(&self.0)
    }
}

impl LabelParen {
    pub fn inner(&self) -> Option<LabelExprNode> {
        child(&self.0)
    }
}

impl LabelAtom {
    pub fn node_label(&self) -> Option<NodeLabel> {
        child(&self.0)
    }

    pub fn rel_type_name(&self) -> Option<RelTypeName> {
        child(&self.0)
    }

    pub fn dynamic_label(&self) -> Option<DynamicLabel> {
        child(&self.0)
    }

    pub fn dynamic_rel_type(&self) -> Option<DynamicRelType> {
        child(&self.0)
    }
}

impl DynamicLabel {
    pub fn expression(&self) -> Option<super::expressions::Expression> {
        child(&self.0)
    }
}

impl DynamicRelType {
    pub fn expression(&self) -> Option<super::expressions::Expression> {
        child(&self.0)
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

    pub fn expression(&self) -> Option<LabelExpression> {
        child(&self.0)
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

impl RelationshipQuantifier {
    pub fn numbers(&self) -> impl Iterator<Item = super::expressions::Literal> {
        self.0
            .children()
            .filter_map(super::expressions::Literal::cast)
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

    pub fn list_literal(&self) -> Option<super::expressions::ListLiteral> {
        child(&self.0)
    }
}

// ============================================================
// RelationshipsPattern — pattern-as-atom in WHERE/RETURN
// ============================================================

#[derive(Clone, Debug)]
pub struct RelationshipsPattern(SyntaxNode);

impl AstNode for RelationshipsPattern {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::RELATIONSHIPS_PATTERN
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(RelationshipsPattern(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RelationshipsPattern {
    pub fn node_pattern(&self) -> Option<NodePattern> {
        child(&self.0)
    }

    pub fn chains(&self) -> AstChildren<PatternElementChain> {
        children(&self.0)
    }
}

#[cfg(test)]
mod tests {
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
