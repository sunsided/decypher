use crate::syntax::{SyntaxKind, SyntaxToken};

use super::traits::AstToken;

#[derive(Clone, Debug)]
pub struct Ident(SyntaxToken);

impl AstToken for Ident {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::IDENT | SyntaxKind::ESCAPED_IDENT)
    }

    fn cast(syntax: SyntaxToken) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Ident(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxToken {
        &self.0
    }
}

impl Ident {
    pub fn is_escaped(&self) -> bool {
        self.0.kind() == SyntaxKind::ESCAPED_IDENT
    }

    pub fn unescape(&self) -> String {
        let text = self.text();
        if self.is_escaped() {
            unwrap_backtick(text)
        } else {
            text.to_string()
        }
    }
}

fn unwrap_backtick(s: &str) -> String {
    let s = s.strip_prefix('`').unwrap_or(s);
    let s = s.strip_suffix('`').unwrap_or(s);
    s.replace("``", "`")
}

#[derive(Clone, Debug)]
pub struct StringToken(SyntaxToken);

impl AstToken for StringToken {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::STRING)
    }

    fn cast(syntax: SyntaxToken) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(StringToken(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxToken {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct IntegerToken(SyntaxToken);

impl AstToken for IntegerToken {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::INTEGER)
    }

    fn cast(syntax: SyntaxToken) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(IntegerToken(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxToken {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct FloatToken(SyntaxToken);

impl AstToken for FloatToken {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::FLOAT)
    }

    fn cast(syntax: SyntaxToken) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(FloatToken(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxToken {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct BooleanToken(SyntaxToken);

impl AstToken for BooleanToken {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW)
    }

    fn cast(syntax: SyntaxToken) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(BooleanToken(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxToken {
        &self.0
    }
}

impl BooleanToken {
    pub fn value(&self) -> bool {
        self.0.kind() == SyntaxKind::TRUE_KW
    }
}

#[derive(Clone, Debug)]
pub struct NullToken(SyntaxToken);

impl AstToken for NullToken {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::NULL_KW)
    }

    fn cast(syntax: SyntaxToken) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(NullToken(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxToken {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn test_ident_unescape_plain() {
        use crate::parser;
        let parse = parser::parse("MATCH (n) RETURN n");
        let idents: Vec<_> = parse
            .tree
            .descendants_with_tokens()
            .filter_map(|el| el.into_token())
            .filter_map(Ident::cast)
            .collect();
        check!(idents.iter().any(|i| i.unescape() == "n"));
    }

    #[test]
    fn test_ident_unescape_backtick() {
        use crate::syntax::{CypherLang, SyntaxNode};
        use rowan::{GreenNodeBuilder, Language};
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(CypherLang::kind_to_raw(SyntaxKind::ESCAPED_IDENT));
        builder.token(
            CypherLang::kind_to_raw(SyntaxKind::ESCAPED_IDENT),
            "`foo``bar`",
        );
        builder.finish_node();
        let green = builder.finish();
        let node: SyntaxNode = rowan::SyntaxNode::new_root(green);
        let token = node.first_token().unwrap();
        let ident = Ident::cast(token).unwrap();
        check!(ident.unescape() == "foo`bar");
    }

    #[test]
    fn test_boolean_token_value() {
        use crate::syntax::{CypherLang, SyntaxNode};
        use rowan::{GreenNodeBuilder, Language};
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(CypherLang::kind_to_raw(SyntaxKind::TRUE_KW));
        builder.token(CypherLang::kind_to_raw(SyntaxKind::TRUE_KW), "true");
        builder.finish_node();
        let green = builder.finish();
        let node: SyntaxNode = rowan::SyntaxNode::new_root(green);
        let token = node.first_token().unwrap();
        let b = BooleanToken::cast(token).unwrap();
        check!(b.value() == true);
    }
}
