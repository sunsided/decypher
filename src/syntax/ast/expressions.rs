use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::support::{child, child_token, child_tokens, children, AstChildren};
use super::traits::AstNode;

// ============================================================
// Expression enum (Option B: collapsed BinaryExpr/UnaryExpr)
// ============================================================

#[derive(Clone, Debug)]
pub enum Expression {
    BinaryExpr(BinaryExpr),
    UnaryExpr(UnaryExpr),
    Atom(Atom),
}

impl AstNode for Expression {
    fn can_cast(kind: SyntaxKind) -> bool {
        BinaryExpr::can_cast(kind) || UnaryExpr::can_cast(kind) || Atom::can_cast(kind)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if BinaryExpr::can_cast(syntax.kind()) {
            return BinaryExpr::cast(syntax).map(Expression::BinaryExpr);
        }
        if UnaryExpr::can_cast(syntax.kind()) {
            return UnaryExpr::cast(syntax).map(Expression::UnaryExpr);
        }
        Atom::cast(syntax).map(Expression::Atom)
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Expression::BinaryExpr(it) => it.syntax(),
            Expression::UnaryExpr(it) => it.syntax(),
            Expression::Atom(it) => it.syntax(),
        }
    }
}

// ============================================================
// BinaryExpr — collapses all precedence-level binary expression kinds
//
// The following SyntaxKinds fold into BinaryExpr:
//   OR_EXPR, XOR_EXPR, AND_EXPR, COMPARISON_EXPR,
//   ADD_SUB_EXPR, MUL_DIV_MOD_EXPR, POWER_EXPR,
//   STRING_LIST_NULL_EXPR, LIST_OP_EXPR, STRING_OP_EXPR,
//   NULL_OP_EXPR, PROPERTY_OR_LABELS_EXPR
// ============================================================

#[derive(Clone, Debug)]
pub struct BinaryExpr(SyntaxNode);

impl AstNode for BinaryExpr {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::OR_EXPR
                | SyntaxKind::XOR_EXPR
                | SyntaxKind::AND_EXPR
                | SyntaxKind::COMPARISON_EXPR
                | SyntaxKind::ADD_SUB_EXPR
                | SyntaxKind::MUL_DIV_MOD_EXPR
                | SyntaxKind::POWER_EXPR
                | SyntaxKind::STRING_LIST_NULL_EXPR
                | SyntaxKind::LIST_OP_EXPR
                | SyntaxKind::STRING_OP_EXPR
                | SyntaxKind::NULL_OP_EXPR
                | SyntaxKind::PROPERTY_OR_LABELS_EXPR
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(BinaryExpr(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl BinaryExpr {
    pub fn op_kind(&self) -> Option<BinOp> {
        match self.0.kind() {
            SyntaxKind::OR_EXPR => Some(BinOp::Or),
            SyntaxKind::XOR_EXPR => Some(BinOp::Xor),
            SyntaxKind::AND_EXPR => Some(BinOp::And),
            SyntaxKind::COMPARISON_EXPR => {
                if child_token(&self.0, SyntaxKind::EQ).is_some() {
                    Some(BinOp::Eq)
                } else if child_token(&self.0, SyntaxKind::NE).is_some() {
                    Some(BinOp::Ne)
                } else if child_token(&self.0, SyntaxKind::LT).is_some() {
                    Some(BinOp::Lt)
                } else if child_token(&self.0, SyntaxKind::GT).is_some() {
                    Some(BinOp::Gt)
                } else if child_token(&self.0, SyntaxKind::LE).is_some() {
                    Some(BinOp::Le)
                } else if child_token(&self.0, SyntaxKind::GE).is_some() {
                    Some(BinOp::Ge)
                } else {
                    None
                }
            }
            SyntaxKind::ADD_SUB_EXPR => {
                if child_token(&self.0, SyntaxKind::PLUS).is_some() {
                    Some(BinOp::Add)
                } else {
                    Some(BinOp::Sub)
                }
            }
            SyntaxKind::MUL_DIV_MOD_EXPR => {
                if child_token(&self.0, SyntaxKind::STAR).is_some() {
                    Some(BinOp::Mul)
                } else if child_token(&self.0, SyntaxKind::SLASH).is_some() {
                    Some(BinOp::Div)
                } else {
                    Some(BinOp::Mod)
                }
            }
            SyntaxKind::POWER_EXPR => Some(BinOp::Power),
            SyntaxKind::STRING_LIST_NULL_EXPR | SyntaxKind::STRING_OP_EXPR => {
                if child_token(&self.0, SyntaxKind::KW_STARTS).is_some() {
                    Some(BinOp::StartsWith)
                } else if child_token(&self.0, SyntaxKind::KW_ENDS).is_some() {
                    Some(BinOp::EndsWith)
                } else if child_token(&self.0, SyntaxKind::KW_CONTAINS).is_some() {
                    Some(BinOp::Contains)
                } else if child_token(&self.0, SyntaxKind::KW_IN).is_some() {
                    Some(BinOp::In)
                } else if child_token(&self.0, SyntaxKind::KW_IS).is_some() {
                    if child_token(&self.0, SyntaxKind::KW_NOT).is_some() {
                        Some(BinOp::IsNotNull)
                    } else {
                        Some(BinOp::IsNull)
                    }
                } else {
                    None
                }
            }
            SyntaxKind::LIST_OP_EXPR => {
                if child_token(&self.0, SyntaxKind::KW_IN).is_some() {
                    Some(BinOp::In)
                } else {
                    Some(BinOp::Index)
                }
            }
            SyntaxKind::NULL_OP_EXPR => {
                if child_token(&self.0, SyntaxKind::KW_NOT).is_some() {
                    Some(BinOp::IsNotNull)
                } else {
                    Some(BinOp::IsNull)
                }
            }
            SyntaxKind::PROPERTY_OR_LABELS_EXPR => {
                if child_token(&self.0, SyntaxKind::DOT).is_some() {
                    Some(BinOp::PropertyLookup)
                } else if child_token(&self.0, SyntaxKind::COLON).is_some() {
                    Some(BinOp::HasLabel)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn lhs(&self) -> Option<Expression> {
        // In the CST, the LHS is a preceding sibling of this node, not a child.
        // e.g. PROJECTION_ITEM { NUMBER_LITERAL "1", ADD_SUB_EXPR "+ 2" }
        // The ADD_SUB_EXPR's lhs is the NUMBER_LITERAL before it.
        self.0
            .prev_sibling()
            .and_then(|s| Expression::cast(s))
            .or_else(|| self.0.children().find_map(Expression::cast))
            .or_else(|| {
                self.0
                    .prev_sibling()
                    .and_then(|s| Variable::cast(s))
                    .map(|v| Expression::Atom(Atom::Variable(v)))
            })
    }

    pub fn rhs(&self) -> Option<Expression> {
        // The RHS is the last expression child of this node.
        // For nested ops like "1 + 2 * 3", the ADD_SUB_EXPR contains:
        //   NUMBER_LITERAL "2", MUL_DIV_MOD_EXPR "* 3"
        // The RHS should be the MUL_DIV_MOD_EXPR.
        self.0.children().filter_map(Expression::cast).last()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    Xor,
    And,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,
    StartsWith,
    EndsWith,
    Contains,
    In,
    IsNull,
    IsNotNull,
    Index,
    PropertyLookup,
    HasLabel,
}

// ============================================================
// UnaryExpr — covers NOT_EXPR and UNARY_ADD_SUB_EXPR
// ============================================================

#[derive(Clone, Debug)]
pub struct UnaryExpr(SyntaxNode);

impl AstNode for UnaryExpr {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::NOT_EXPR | SyntaxKind::UNARY_ADD_SUB_EXPR)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(UnaryExpr(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl UnaryExpr {
    pub fn op(&self) -> Option<UnOp> {
        if self.0.kind() == SyntaxKind::NOT_EXPR {
            Some(UnOp::Not)
        } else if child_token(&self.0, SyntaxKind::PLUS).is_some() {
            Some(UnOp::Pos)
        } else if child_token(&self.0, SyntaxKind::MINUS).is_some() {
            Some(UnOp::Neg)
        } else {
            None
        }
    }

    pub fn operand(&self) -> Option<Expression> {
        // In the flat CST, the operand is the LAST expression child.
        // e.g. NOT n.active produces:
        //   NOT_EXPR
        //     └── KW_NOT
        //     └── VARIABLE (n)
        //     └── PROPERTY_LOOKUP (.active)  ← this is the root operand
        self.0.children().filter_map(Expression::cast).last()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Pos,
    Neg,
}

// ============================================================
// Atom enum
// ============================================================

#[derive(Clone, Debug)]
pub enum Atom {
    Literal(Literal),
    Variable(Variable),
    Parameter(Parameter),
    FunctionInvocation(FunctionInvocation),
    Parenthesized(ParenthesizedExpr),
    Case(CaseExpr),
    ListLiteral(ListLiteral),
    MapLiteral(MapLiteral),
    ListComprehension(ListComprehension),
    PatternComprehension(PatternComprehension),
    FilterExpression(FilterExpression),
    ExistsSubquery(ExistsSubquery),
    MapProjection(MapProjection),
    ImplicitProcedureInvocation(ImplicitProcedureInvocation),
    PropertyLookup(PropertyLookup),
    Null(NullLiteral),
}

impl AstNode for Atom {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LITERAL
                | SyntaxKind::NUMBER_LITERAL
                | SyntaxKind::STRING_LITERAL
                | SyntaxKind::BOOLEAN_LITERAL
                | SyntaxKind::NULL_KW
                | SyntaxKind::VARIABLE
                | SyntaxKind::PARAMETER
                | SyntaxKind::FUNCTION_INVOCATION
                | SyntaxKind::PARENTHESIZED_EXPR
                | SyntaxKind::CASE_EXPR
                | SyntaxKind::LIST_LITERAL
                | SyntaxKind::MAP_LITERAL
                | SyntaxKind::LIST_COMPREHENSION
                | SyntaxKind::PATTERN_COMPREHENSION
                | SyntaxKind::FILTER_EXPRESSION
                | SyntaxKind::EXISTS_SUBQUERY
                | SyntaxKind::MAP_PROJECTION
                | SyntaxKind::IMPLICIT_PROCEDURE_INVOCATION
                | SyntaxKind::PROPERTY_LOOKUP
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::LITERAL
            | SyntaxKind::NUMBER_LITERAL
            | SyntaxKind::STRING_LITERAL
            | SyntaxKind::BOOLEAN_LITERAL => Literal::cast(syntax).map(Atom::Literal),
            SyntaxKind::NULL_KW => NullLiteral::cast(syntax).map(Atom::Null),
            SyntaxKind::VARIABLE => Variable::cast(syntax).map(Atom::Variable),
            SyntaxKind::PARAMETER => Parameter::cast(syntax).map(Atom::Parameter),
            SyntaxKind::FUNCTION_INVOCATION => {
                FunctionInvocation::cast(syntax).map(Atom::FunctionInvocation)
            }
            SyntaxKind::PARENTHESIZED_EXPR => {
                ParenthesizedExpr::cast(syntax).map(Atom::Parenthesized)
            }
            SyntaxKind::CASE_EXPR => CaseExpr::cast(syntax).map(Atom::Case),
            SyntaxKind::LIST_LITERAL => ListLiteral::cast(syntax).map(Atom::ListLiteral),
            SyntaxKind::MAP_LITERAL => MapLiteral::cast(syntax).map(Atom::MapLiteral),
            SyntaxKind::LIST_COMPREHENSION => {
                ListComprehension::cast(syntax).map(Atom::ListComprehension)
            }
            SyntaxKind::PATTERN_COMPREHENSION => {
                PatternComprehension::cast(syntax).map(Atom::PatternComprehension)
            }
            SyntaxKind::FILTER_EXPRESSION => {
                FilterExpression::cast(syntax).map(Atom::FilterExpression)
            }
            SyntaxKind::EXISTS_SUBQUERY => ExistsSubquery::cast(syntax).map(Atom::ExistsSubquery),
            SyntaxKind::MAP_PROJECTION => MapProjection::cast(syntax).map(Atom::MapProjection),
            SyntaxKind::IMPLICIT_PROCEDURE_INVOCATION => {
                ImplicitProcedureInvocation::cast(syntax).map(Atom::ImplicitProcedureInvocation)
            }
            SyntaxKind::PROPERTY_LOOKUP => PropertyLookup::cast(syntax).map(Atom::PropertyLookup),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Atom::Literal(it) => it.syntax(),
            Atom::Variable(it) => it.syntax(),
            Atom::Parameter(it) => it.syntax(),
            Atom::FunctionInvocation(it) => it.syntax(),
            Atom::Parenthesized(it) => it.syntax(),
            Atom::Case(it) => it.syntax(),
            Atom::ListLiteral(it) => it.syntax(),
            Atom::MapLiteral(it) => it.syntax(),
            Atom::ListComprehension(it) => it.syntax(),
            Atom::PatternComprehension(it) => it.syntax(),
            Atom::FilterExpression(it) => it.syntax(),
            Atom::ExistsSubquery(it) => it.syntax(),
            Atom::MapProjection(it) => it.syntax(),
            Atom::ImplicitProcedureInvocation(it) => it.syntax(),
            Atom::PropertyLookup(it) => it.syntax(),
            Atom::Null(it) => it.syntax(),
        }
    }
}

// ============================================================
// Literal
// ============================================================

#[derive(Clone, Debug)]
pub enum Literal {
    Number(NumberLiteral),
    String(StringLiteral),
    Boolean(BooleanLiteral),
    Null(NullLiteral),
}

impl AstNode for Literal {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LITERAL
                | SyntaxKind::NUMBER_LITERAL
                | SyntaxKind::STRING_LITERAL
                | SyntaxKind::BOOLEAN_LITERAL
                | SyntaxKind::NULL_KW
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::LITERAL | SyntaxKind::NUMBER_LITERAL => {
                NumberLiteral::cast(syntax).map(Literal::Number)
            }
            SyntaxKind::STRING_LITERAL => StringLiteral::cast(syntax).map(Literal::String),
            SyntaxKind::BOOLEAN_LITERAL => BooleanLiteral::cast(syntax).map(Literal::Boolean),
            SyntaxKind::NULL_KW => Some(Literal::Null(NullLiteral(syntax))),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Literal::Number(it) => it.syntax(),
            Literal::String(it) => it.syntax(),
            Literal::Boolean(it) => it.syntax(),
            Literal::Null(it) => it.syntax(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NumberLiteral(SyntaxNode);

impl AstNode for NumberLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::NUMBER_LITERAL | SyntaxKind::LITERAL)
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(NumberLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NumberLiteral {
    pub fn token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::INTEGER)
            .or_else(|| child_token(&self.0, SyntaxKind::FLOAT))
    }
}

#[derive(Clone, Debug)]
pub struct StringLiteral(SyntaxNode);

impl AstNode for StringLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::STRING_LITERAL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(StringLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl StringLiteral {
    pub fn token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::STRING)
    }
}

#[derive(Clone, Debug)]
pub struct BooleanLiteral(SyntaxNode);

impl AstNode for BooleanLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BOOLEAN_LITERAL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(BooleanLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl BooleanLiteral {
    pub fn token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::TRUE_KW)
            .or_else(|| child_token(&self.0, SyntaxKind::FALSE_KW))
    }

    pub fn value(&self) -> bool {
        child_token(&self.0, SyntaxKind::TRUE_KW).is_some()
    }
}

#[derive(Clone, Debug)]
pub struct NullLiteral(SyntaxNode);

impl AstNode for NullLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::NULL_KW
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(NullLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ============================================================
// Variable (atom-level, separate from top-level Variable)
// ============================================================

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
    pub fn name(&self) -> Option<super::top_level::SymbolicName> {
        child(&self.0)
    }
}

// ============================================================
// Parameter
// ============================================================

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

impl Parameter {
    /// Returns the parameter name token ($name -> "name") or the numeric
    /// index token ($1 -> "1") depending on the form used.
    pub fn name_token(&self) -> Option<SyntaxToken> {
        self.0.children_with_tokens().find_map(|c| {
            c.into_token()
                .filter(|t| matches!(t.kind(), SyntaxKind::IDENT | SyntaxKind::INTEGER))
        })
    }
}

// ============================================================
// FunctionInvocation
// ============================================================

#[derive(Clone, Debug)]
pub struct FunctionInvocation(SyntaxNode);

impl AstNode for FunctionInvocation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FUNCTION_INVOCATION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(FunctionInvocation(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl FunctionInvocation {
    pub fn name(&self) -> Option<FunctionName> {
        // Two patterns from the grammar:
        // 1. KW_COUNT (and keywords) → FUNCTION_NAME is a CHILD of FUNCTION_INVOCATION
        // 2. IDENT identifiers → VARIABLE is a PREV_SIBLING (checkpoint set after it)
        self.0
            .children()
            .find_map(FunctionName::cast)
            .or_else(|| self.0.prev_sibling().and_then(FunctionName::cast))
    }

    pub fn arguments(&self) -> impl Iterator<Item = Expression> {
        self.0
            .children()
            .filter(|n| {
                !matches!(
                    n.kind(),
                    SyntaxKind::FUNCTION_NAME | SyntaxKind::NAMESPACE | SyntaxKind::VARIABLE
                )
            })
            .filter_map(Expression::cast)
    }

    pub fn distinct_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_DISTINCT)
    }

    pub fn star_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::STAR)
    }
}

#[derive(Clone, Debug)]
pub struct FunctionName(SyntaxNode);

impl AstNode for FunctionName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FUNCTION_NAME || kind == SyntaxKind::VARIABLE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(FunctionName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl FunctionName {
    pub fn namespace(&self) -> Option<Namespace> {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::SYMBOLIC_NAME)
            .take(1)
            .next()
            .and_then(|n| {
                if self
                    .0
                    .children()
                    .filter(|n| n.kind() == SyntaxKind::SYMBOLIC_NAME)
                    .count()
                    > 1
                {
                    Some(Namespace(n))
                } else {
                    None
                }
            })
    }

    pub fn symbolic_names(&self) -> impl Iterator<Item = super::top_level::SymbolicName> {
        self.0
            .children()
            .filter_map(super::top_level::SymbolicName::cast)
    }
}

#[derive(Clone, Debug)]
pub struct Namespace(SyntaxNode);

impl AstNode for Namespace {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::NAMESPACE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Namespace(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ============================================================
// ParenthesizedExpr
// ============================================================

#[derive(Clone, Debug)]
pub struct ParenthesizedExpr(SyntaxNode);

impl AstNode for ParenthesizedExpr {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PARENTHESIZED_EXPR
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ParenthesizedExpr(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ParenthesizedExpr {
    pub fn expr(&self) -> Option<Expression> {
        // In the flat CST, the expression is the LAST expression child.
        // e.g. (1 + 2) produces:
        //   PARENTHESIZED_EXPR
        //     └── L_PAREN
        //     └── NUMBER_LITERAL (1)
        //     └── ADD_SUB_EXPR (+ 2)  ← this is the root
        //     └── R_PAREN
        self.0.children().filter_map(Expression::cast).last()
    }
}

// ============================================================
// CaseExpr
// ============================================================

#[derive(Clone, Debug)]
pub struct CaseExpr(SyntaxNode);

impl AstNode for CaseExpr {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CASE_EXPR
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(CaseExpr(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CaseExpr {
    pub fn value(&self) -> Option<Expression> {
        // Scrutinee is the Expression appearing after CASE but before the
        // first WHEN or ELSE keyword. KW_WHEN / KW_ELSE are tokens, so we
        // must iterate children_with_tokens to detect them.
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if let Some(tok) = child.as_token() {
                if matches!(tok.kind(), SyntaxKind::KW_WHEN | SyntaxKind::KW_ELSE) {
                    break;
                }
                continue;
            }
            if let Some(node) = child.as_node() {
                // CASE_ALTERNATIVE (the first WHEN branch) is also a child;
                // stop before consuming it so alternatives don't get mistaken
                // for the scrutinee.
                if node.kind() == SyntaxKind::CASE_ALTERNATIVE {
                    break;
                }
                if let Some(e) = Expression::cast(node.clone()) {
                    last = Some(e);
                }
            }
        }
        last
    }

    pub fn alternatives(&self) -> AstChildren<CaseAlternative> {
        children(&self.0)
    }

    pub fn else_expr(&self) -> Option<Expression> {
        // ELSE is a token, so use children_with_tokens. The expression that
        // follows the ELSE token is the default.
        let mut seen_else = false;
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if let Some(tok) = child.as_token() {
                if tok.kind() == SyntaxKind::KW_ELSE {
                    seen_else = true;
                }
                continue;
            }
            if seen_else {
                if let Some(node) = child.as_node() {
                    if let Some(e) = Expression::cast(node.clone()) {
                        last = Some(e);
                    }
                }
            }
        }
        last
    }
}

#[derive(Clone, Debug)]
pub struct CaseAlternative(SyntaxNode);

impl AstNode for CaseAlternative {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CASE_ALTERNATIVE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(CaseAlternative(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CaseAlternative {
    pub fn when_expr(&self) -> Option<Expression> {
        // Return the LAST Expression child appearing before KW_THEN. The CST
        // stores chains like `n.active` as flat siblings (VARIABLE, PROPERTY_LOOKUP),
        // and the rightmost node carries the composition via prev_sibling().
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if child
                .as_token()
                .map_or(false, |t| t.kind() == SyntaxKind::KW_THEN)
            {
                break;
            }
            if let Some(node) = child.as_node() {
                if let Some(e) = Expression::cast(node.clone()) {
                    last = Some(e);
                }
            }
        }
        last
    }

    pub fn then_expr(&self) -> Option<Expression> {
        // Return the LAST Expression child appearing after KW_THEN.
        let mut seen_then = false;
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if let Some(tok) = child.as_token() {
                if tok.kind() == SyntaxKind::KW_THEN {
                    seen_then = true;
                }
                continue;
            }
            if seen_then {
                if let Some(node) = child.as_node() {
                    if let Some(expr) = Expression::cast(node.clone()) {
                        last = Some(expr);
                    }
                }
            }
        }
        last
    }
}

// ============================================================
// ListLiteral
// ============================================================

#[derive(Clone, Debug)]
pub struct ListLiteral(SyntaxNode);

impl AstNode for ListLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LIST_LITERAL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ListLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ListLiteral {
    pub fn elements(&self) -> impl Iterator<Item = Expression> {
        self.0.children().filter_map(Expression::cast)
    }
}

// ============================================================
// MapLiteral
// ============================================================

#[derive(Clone, Debug)]
pub struct MapLiteral(SyntaxNode);

impl AstNode for MapLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MAP_LITERAL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MapLiteral(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MapLiteral {
    pub fn entries(&self) -> AstChildren<MapEntry> {
        children(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct MapEntry(SyntaxNode);

impl AstNode for MapEntry {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MAP_ENTRY
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MapEntry(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MapEntry {
    pub fn key(&self) -> Option<PropertyKeyName> {
        child(&self.0)
    }

    pub fn value(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| {
                n.kind() != SyntaxKind::PROPERTY_KEY_NAME && n.kind() != SyntaxKind::SYMBOLIC_NAME
            })
            .find_map(Expression::cast)
    }
}

#[derive(Clone, Debug)]
pub struct PropertyKeyName(SyntaxNode);

impl AstNode for PropertyKeyName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROPERTY_KEY_NAME
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PropertyKeyName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PropertyKeyName {
    pub fn symbolic_name(&self) -> Option<super::top_level::SymbolicName> {
        child(&self.0)
    }
}

// ============================================================
// ListComprehension
// ============================================================

#[derive(Clone, Debug)]
pub struct ListComprehension(SyntaxNode);

impl AstNode for ListComprehension {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LIST_COMPREHENSION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ListComprehension(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ListComprehension {
    pub fn filter(&self) -> Option<FilterExpression> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<Expression> {
        self.0
            .children()
            .skip_while(|n| !matches!(n.kind(), SyntaxKind::PIPE | SyntaxKind::FILTER_EXPRESSION))
            .filter_map(Expression::cast)
            .last()
    }
}

// ============================================================
// PatternComprehension
// ============================================================

#[derive(Clone, Debug)]
pub struct PatternComprehension(SyntaxNode);

impl AstNode for PatternComprehension {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PATTERN_COMPREHENSION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PatternComprehension(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternComprehension {
    pub fn variable(&self) -> Option<Variable> {
        self.0
            .children()
            .take_while(|n| {
                n.kind() != SyntaxKind::RELATIONSHIPS_PATTERN
                    && n.kind() != SyntaxKind::NODE_PATTERN
                    && n.kind() != SyntaxKind::PATTERN_ELEMENT_CHAIN
            })
            .find_map(Variable::cast)
    }

    pub fn pattern(&self) -> Option<SyntaxNode> {
        self.0.children().find(|n| {
            matches!(
                n.kind(),
                SyntaxKind::RELATIONSHIPS_PATTERN
                    | SyntaxKind::NODE_PATTERN
                    | SyntaxKind::PATTERN_ELEMENT_CHAIN
            )
        })
    }

    pub fn where_clause(&self) -> Option<super::clauses::WhereClause> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<Expression> {
        self.0
            .children()
            .skip_while(|n| n.kind() != SyntaxKind::PIPE)
            .filter_map(Expression::cast)
            .last()
    }
}

// ============================================================
// FilterExpression
// ============================================================

#[derive(Clone, Debug)]
pub struct FilterExpression(SyntaxNode);

impl AstNode for FilterExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FILTER_EXPRESSION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(FilterExpression(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl FilterExpression {
    pub fn id_in_coll(&self) -> Option<IdInColl> {
        child(&self.0)
    }

    pub fn where_clause(&self) -> Option<super::clauses::WhereClause> {
        self.0
            .children()
            .filter(|n| n.kind() != SyntaxKind::ID_IN_COLL)
            .find_map(|n| super::clauses::WhereClause::cast(n))
    }
}

// ============================================================
// IdInColl
// ============================================================

#[derive(Clone, Debug)]
pub struct IdInColl(SyntaxNode);

impl AstNode for IdInColl {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ID_IN_COLL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(IdInColl(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl IdInColl {
    pub fn variable(&self) -> Option<Variable> {
        child(&self.0)
    }

    pub fn collection(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| n.kind() != SyntaxKind::VARIABLE && n.kind() != SyntaxKind::SYMBOLIC_NAME)
            .find_map(Expression::cast)
    }
}

// ============================================================
// ExistsSubquery
// ============================================================

#[derive(Clone, Debug)]
pub struct ExistsSubquery(SyntaxNode);

impl AstNode for ExistsSubquery {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::EXISTS_SUBQUERY
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ExistsSubquery(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ExistsSubquery {
    pub fn pattern(&self) -> Option<SyntaxNode> {
        self.0.children().find(|n| {
            matches!(
                n.kind(),
                SyntaxKind::NODE_PATTERN
                    | SyntaxKind::RELATIONSHIPS_PATTERN
                    | SyntaxKind::PATTERN_ELEMENT_CHAIN
            )
        })
    }

    pub fn where_clause(&self) -> Option<super::clauses::WhereClause> {
        child(&self.0)
    }
}

// ============================================================
// MapProjection
// ============================================================

#[derive(Clone, Debug)]
pub struct MapProjection(SyntaxNode);

impl AstNode for MapProjection {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MAP_PROJECTION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MapProjection(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MapProjection {
    pub fn variable(&self) -> Option<Variable> {
        child(&self.0)
    }

    pub fn items(&self) -> impl Iterator<Item = MapProjectionItem> {
        self.0.children().filter_map(MapProjectionItem::cast)
    }
}

// ============================================================
// MapProjectionItem
// ============================================================

#[derive(Clone, Debug)]
pub struct MapProjectionItem(SyntaxNode);

impl AstNode for MapProjectionItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::MAP_PROJECTION_ITEM
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(MapProjectionItem(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MapProjectionItem {
    pub fn is_star(&self) -> bool {
        child_token(&self.0, SyntaxKind::STAR).is_some()
    }

    pub fn property_name(&self) -> Option<PropertyKeyName> {
        child(&self.0)
    }

    pub fn expression(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| {
                n.kind() != SyntaxKind::PROPERTY_KEY_NAME && n.kind() != SyntaxKind::SYMBOLIC_NAME
            })
            .find_map(Expression::cast)
    }
}

// ============================================================
// PropertyLookup
// ============================================================

#[derive(Clone, Debug)]
pub struct PropertyLookup(SyntaxNode);

impl AstNode for PropertyLookup {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROPERTY_LOOKUP
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PropertyLookup(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PropertyLookup {
    pub fn key(&self) -> Option<PropertyKeyName> {
        child(&self.0)
    }

    pub fn base(&self) -> Option<Expression> {
        // In the flat CST, the base is the preceding sibling of this PROPERTY_LOOKUP.
        // e.g. WHERE n.name = 'Alice' produces:
        //   WHERE_CLAUSE
        //     └── VARIABLE (n)
        //     └── PROPERTY_LOOKUP (.name)  ← base is VARIABLE
        //     └── COMPARISON_EXPR
        self.0.prev_sibling().and_then(Expression::cast)
    }
}

// ============================================================
// PropertyExpression
// ============================================================

#[derive(Clone, Debug)]
pub struct PropertyExpression(SyntaxNode);

impl AstNode for PropertyExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROPERTY_EXPRESSION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(PropertyExpression(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ============================================================
// ProcedureName — namespace + name for procedure calls
// ============================================================

#[derive(Clone, Debug)]
pub struct ProcedureName(SyntaxNode);

impl AstNode for ProcedureName {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROCEDURE_NAME
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ProcedureName(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ProcedureName {
    pub fn namespace(&self) -> Option<Namespace> {
        let names: Vec<_> = self
            .0
            .children()
            .filter(|n| n.kind() == SyntaxKind::SYMBOLIC_NAME)
            .collect();
        if names.len() > 1 {
            names.first().cloned().map(|n| Namespace(n.clone()))
        } else {
            None
        }
    }

    pub fn symbolic_names(&self) -> impl Iterator<Item = super::top_level::SymbolicName> {
        self.0
            .children()
            .filter_map(super::top_level::SymbolicName::cast)
    }
}

// ============================================================
// ImplicitProcedureInvocation — CALL procName without parens
// ============================================================

#[derive(Clone, Debug)]
pub struct ImplicitProcedureInvocation(SyntaxNode);

impl AstNode for ImplicitProcedureInvocation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IMPLICIT_PROCEDURE_INVOCATION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ImplicitProcedureInvocation(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ImplicitProcedureInvocation {
    pub fn procedure_name(&self) -> Option<ProcedureName> {
        child(&self.0)
    }
}

// ============================================================
// ExplicitProcedureInvocation — CALL procName(args)
// ============================================================

#[derive(Clone, Debug)]
pub struct ExplicitProcedureInvocation(SyntaxNode);

impl AstNode for ExplicitProcedureInvocation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::EXPLICIT_PROCEDURE_INVOCATION
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ExplicitProcedureInvocation(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ExplicitProcedureInvocation {
    pub fn procedure_name(&self) -> Option<ProcedureName> {
        child(&self.0)
    }

    pub fn arguments(&self) -> impl Iterator<Item = Expression> {
        self.0
            .children()
            .filter(|n| n.kind() != SyntaxKind::PROCEDURE_NAME && n.kind() != SyntaxKind::NAMESPACE)
            .filter_map(Expression::cast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::syntax::ast::traits::AstNode;
    use assert2::check;

    fn find_return(
        source: &super::super::top_level::SourceFile,
    ) -> super::super::clauses::ReturnClause {
        for clause in source.statements().next().unwrap().clauses() {
            if let super::super::clauses::Clause::Return(r) = clause {
                return r;
            }
        }
        panic!("no return clause found");
    }

    #[test]
    fn test_binary_expr_add() {
        let parse = parser::parse("RETURN 1 + 2");
        let source = super::super::top_level::SourceFile::cast(parse.tree.clone()).unwrap();
        let ret = find_return(&source);
        let proj = ret.projection_body().unwrap();
        let item = proj.items().next().unwrap();
        let expr = item.expr().unwrap();
        let bin = match expr {
            Expression::BinaryExpr(b) => b,
            other => panic!("expected BinaryExpr, got {:?}", other.syntax().kind()),
        };
        check!(bin.op_kind() == Some(BinOp::Add));
    }

    #[test]
    fn test_binary_expr_mul_precedence() {
        let parse = parser::parse("RETURN 1 + 2 * 3");
        let source = super::super::top_level::SourceFile::cast(parse.tree.clone()).unwrap();
        let ret = find_return(&source);
        let proj = ret.projection_body().unwrap();
        let item = proj.items().next().unwrap();
        let expr = item.expr().unwrap();
        // Top-level should be Add, RHS should be Mul
        let add = match expr {
            Expression::BinaryExpr(b) => b,
            _ => panic!("expected BinaryExpr at top"),
        };
        check!(add.op_kind() == Some(BinOp::Add));
        let rhs = add.rhs().unwrap();
        let mul = match rhs {
            Expression::BinaryExpr(b) => b,
            _ => panic!("expected BinaryExpr on rhs"),
        };
        check!(mul.op_kind() == Some(BinOp::Mul));
    }

    #[test]
    fn test_unary_expr_not() {
        let parse = parser::parse("RETURN NOT true");
        let source = super::super::top_level::SourceFile::cast(parse.tree).unwrap();
        let ret = find_return(&source);
        let proj = ret.projection_body().unwrap();
        let item = proj.items().next().unwrap();
        let expr = item.expr().unwrap();
        let unary = match expr {
            Expression::UnaryExpr(u) => u,
            _ => panic!("expected UnaryExpr"),
        };
        check!(unary.op() == Some(UnOp::Not));
    }

    #[test]
    fn test_expression_can_cast() {
        use crate::syntax::{CypherLang, SyntaxNode};
        use rowan::{GreenNodeBuilder, Language};
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(CypherLang::kind_to_raw(SyntaxKind::SOURCE_FILE));
        builder.finish_node();
        let green = builder.finish();
        let node: SyntaxNode = rowan::SyntaxNode::new_root(green);
        check!(Expression::can_cast(node.kind()) == false);
        check!(BinaryExpr::can_cast(SyntaxKind::OR_EXPR) == true);
        check!(BinaryExpr::can_cast(SyntaxKind::ATOM) == false);
        check!(UnaryExpr::can_cast(SyntaxKind::NOT_EXPR) == true);
        check!(Atom::can_cast(SyntaxKind::VARIABLE) == true);
    }
}
