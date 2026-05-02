use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use super::expressions::{
    ExplicitProcedureInvocation, Expression, ImplicitProcedureInvocation, NumberLiteral,
};
use super::patterns::Pattern;
use super::projection::ProjectionBody;
use super::support::{child, child_token, child_tokens, children, AstChildren};
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
    Foreach(ForeachClause),
    StandaloneCall(StandaloneCall),
    InQueryCall(InQueryCall),
    CallSubquery(CallSubqueryClause),
    Show(ShowClause),
    Use(UseClause),
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
            || ForeachClause::can_cast(kind)
            || StandaloneCall::can_cast(kind)
            || InQueryCall::can_cast(kind)
            || CallSubqueryClause::can_cast(kind)
            || ShowClause::can_cast(kind)
            || UseClause::can_cast(kind)
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
        if ForeachClause::can_cast(syntax.kind()) {
            return ForeachClause::cast(syntax).map(Clause::Foreach);
        }
        if StandaloneCall::can_cast(syntax.kind()) {
            return StandaloneCall::cast(syntax).map(Clause::StandaloneCall);
        }
        if InQueryCall::can_cast(syntax.kind()) {
            return InQueryCall::cast(syntax).map(Clause::InQueryCall);
        }
        if CallSubqueryClause::can_cast(syntax.kind()) {
            return CallSubqueryClause::cast(syntax).map(Clause::CallSubquery);
        }
        if ShowClause::can_cast(syntax.kind()) {
            return ShowClause::cast(syntax).map(Clause::Show);
        }
        if UseClause::can_cast(syntax.kind()) {
            return UseClause::cast(syntax).map(Clause::Use);
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
            Clause::Foreach(it) => it.syntax(),
            Clause::StandaloneCall(it) => it.syntax(),
            Clause::InQueryCall(it) => it.syntax(),
            Clause::CallSubquery(it) => it.syntax(),
            Clause::Show(it) => it.syntax(),
            Clause::Use(it) => it.syntax(),
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
        // In the flat CST, the property expression is the LAST expression-like
        // child BEFORE the `=` / `+=` operator. e.g. SET n.name = 'Bob' produces:
        //   SET_ITEM
        //     └── VARIABLE (n)
        //     └── PROPERTY_LOOKUP (.name)  ← this is the root
        //     └── EQ
        //     └── STRING_LITERAL
        // We must not consume nodes after the operator: the RHS can also
        // contain VARIABLE nodes (e.g. `timestamp` in `timestamp()`).
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if let Some(tok) = child.as_token() {
                if matches!(tok.kind(), SyntaxKind::EQ | SyntaxKind::PLUSEQ) {
                    break;
                }
                continue;
            }
            if let Some(node) = child.as_node() {
                if matches!(
                    node.kind(),
                    SyntaxKind::VARIABLE
                        | SyntaxKind::PROPERTY_LOOKUP
                        | SyntaxKind::PROPERTY_OR_LABELS_EXPR
                        | SyntaxKind::PROPERTY_EXPRESSION
                ) {
                    if let Some(e) = Expression::cast(node.clone()) {
                        last = Some(e);
                    }
                }
            }
        }
        last
    }

    pub fn eq_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::EQ)
    }

    pub fn plus_eq_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::PLUSEQ)
    }

    pub fn value_expr(&self) -> Option<Expression> {
        // The value expression appears AFTER the `=` / `+=` operator.
        // Return the LAST Expression-castable node after that operator so
        // that flat chains like `VARIABLE timestamp` + `FUNCTION_INVOCATION ()`
        // resolve to the composite FunctionInvocation via prev_sibling.
        let mut seen_op = false;
        let mut last: Option<Expression> = None;
        for child in self.0.children_with_tokens() {
            if let Some(tok) = child.as_token() {
                if matches!(tok.kind(), SyntaxKind::EQ | SyntaxKind::PLUSEQ) {
                    seen_op = true;
                }
                continue;
            }
            if seen_op {
                if let Some(node) = child.as_node() {
                    if let Some(e) = Expression::cast(node.clone()) {
                        last = Some(e);
                    }
                }
            }
        }
        last
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
        // In the flat CST, the property expression is the LAST expression-like child.
        // e.g. REMOVE n.name produces:
        //   REMOVE_ITEM
        //     └── VARIABLE (n)
        //     └── PROPERTY_LOOKUP (.name)  ← this is the root
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
            .last()
            .and_then(Expression::cast)
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
        // In the Pratt parser, the top-level operator is the LAST expression child.
        // e.g. WHERE n.name = 'Alice' produces:
        //   WHERE_CLAUSE
        //     └── VARIABLE (n)
        //     └── PROPERTY_LOOKUP (.name)
        //     └── COMPARISON_EXPR (= 'Alice')  ← this is the root
        self.0.children().filter_map(Expression::cast).last()
    }
}

// ============================================================
// ForeachClause
// ============================================================

#[derive(Clone, Debug)]
pub struct ForeachClause(SyntaxNode);

impl AstNode for ForeachClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FOREACH_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ForeachClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ForeachClause {
    pub fn variable(&self) -> Option<super::top_level::Variable> {
        child(&self.0)
    }

    pub fn list(&self) -> Option<Expression> {
        self.0
            .children()
            .filter(|n| n.kind() != SyntaxKind::VARIABLE && n.kind() != SyntaxKind::SYMBOLIC_NAME)
            .take_while(|n| n.kind() != SyntaxKind::PIPE)
            .find_map(Expression::cast)
    }

    pub fn clauses(&self) -> AstChildren<Clause> {
        children(&self.0)
    }
}

// ============================================================
// StandaloneCall — CALL proc() [YIELD ...]
// ============================================================

#[derive(Clone, Debug)]
pub struct StandaloneCall(SyntaxNode);

impl AstNode for StandaloneCall {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::STANDALONE_CALL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(StandaloneCall(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl StandaloneCall {
    pub fn explicit_invocation(&self) -> Option<super::expressions::ExplicitProcedureInvocation> {
        self.0.children().find_map(|n| {
            if n.kind() == SyntaxKind::EXPLICIT_PROCEDURE_INVOCATION {
                super::expressions::ExplicitProcedureInvocation::cast(n)
            } else {
                None
            }
        })
    }

    pub fn implicit_invocation(&self) -> Option<super::expressions::ImplicitProcedureInvocation> {
        self.0.children().find_map(|n| {
            if n.kind() == SyntaxKind::IMPLICIT_PROCEDURE_INVOCATION {
                super::expressions::ImplicitProcedureInvocation::cast(n)
            } else {
                None
            }
        })
    }

    pub fn yield_items(&self) -> Option<YieldItems> {
        child(&self.0)
    }
}

// ============================================================
// InQueryCall — standalone YIELD (in-query call variant)
// ============================================================

#[derive(Clone, Debug)]
pub struct InQueryCall(SyntaxNode);

impl AstNode for InQueryCall {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IN_QUERY_CALL
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(InQueryCall(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl InQueryCall {
    pub fn yield_items(&self) -> Option<YieldItems> {
        child(&self.0)
    }
}

// ============================================================
// YieldItems — YIELD * or YIELD field1, field2 [WHERE ...]
// ============================================================

#[derive(Clone, Debug)]
pub struct YieldItems(SyntaxNode);

impl AstNode for YieldItems {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::YIELD_ITEMS
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(YieldItems(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl YieldItems {
    pub fn star_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::STAR)
    }

    pub fn items(&self) -> AstChildren<YieldItem> {
        children(&self.0)
    }

    pub fn where_expr(&self) -> Option<Expression> {
        self.0
            .children_with_tokens()
            .skip_while(|el| !matches!(el.as_token().map(|t| t.kind()), Some(SyntaxKind::KW_WHERE)))
            .filter_map(|el| el.into_node())
            .find_map(Expression::cast)
    }
}

// ============================================================
// YieldItem — a single yield field [AS alias]
// ============================================================

#[derive(Clone, Debug)]
pub struct YieldItem(SyntaxNode);

impl AstNode for YieldItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::YIELD_ITEM
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(YieldItem(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl YieldItem {
    pub fn field_name(&self) -> Option<ProcedureResultField> {
        child(&self.0)
    }

    pub fn alias(&self) -> Option<super::top_level::Variable> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ProcedureResultField(SyntaxNode);

impl AstNode for ProcedureResultField {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PROCEDURE_RESULT_FIELD
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ProcedureResultField(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ProcedureResultField {
    pub fn symbolic_name(&self) -> Option<super::top_level::SymbolicName> {
        child(&self.0)
    }
}

// ============================================================
// CallSubqueryClause — CALL { ... } [IN TRANSACTIONS ...]
// ============================================================

#[derive(Clone, Debug)]
pub struct CallSubqueryClause(SyntaxNode);

impl AstNode for CallSubqueryClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CALL_SUBQUERY_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(CallSubqueryClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CallSubqueryClause {
    pub fn in_transactions(&self) -> Option<InTransactions> {
        child(&self.0)
    }

    pub fn inner_clauses(&self) -> impl Iterator<Item = Clause> {
        self.0.children().filter_map(Clause::cast)
    }

    pub fn inner_unions(&self) -> AstChildren<super::top_level::Union> {
        children(&self.0)
    }
}

// ============================================================
// InTransactions — IN TRANSACTIONS [OF n ROWS] [ON ERROR ...]
// ============================================================

#[derive(Clone, Debug)]
pub struct InTransactions(SyntaxNode);

impl AstNode for InTransactions {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IN_TRANSACTIONS
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(InTransactions(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl InTransactions {
    pub fn rows_expr(&self) -> Option<super::expressions::NumberLiteral> {
        child(&self.0)
    }

    pub fn on_error_action(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::KW_CONTINUE)
            .or_else(|| child_token(&self.0, SyntaxKind::KW_BREAK))
            .or_else(|| child_token(&self.0, SyntaxKind::KW_FAIL))
    }
}

// ============================================================
// ShowClause — SHOW <kind> [YIELD ...] [RETURN ...]
// ============================================================

#[derive(Clone, Debug)]
pub struct ShowClause(SyntaxNode);

impl AstNode for ShowClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SHOW_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ShowClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ShowClause {
    pub fn kind(&self) -> Option<ShowKind> {
        child(&self.0)
    }

    pub fn show_return(&self) -> Option<ShowReturn> {
        child(&self.0)
    }

    pub fn return_clause(&self) -> Option<ReturnClause> {
        child(&self.0)
    }
}

// ============================================================
// ShowKind — wraps the SHOW target keyword
// ============================================================

#[derive(Clone, Debug)]
pub struct ShowKind(SyntaxNode);

impl AstNode for ShowKind {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SHOW_KIND
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ShowKind(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ============================================================
// ShowReturn — SHOW's YIELD section (star or fields + optional WHERE)
// ============================================================

#[derive(Clone, Debug)]
pub struct ShowReturn(SyntaxNode);

impl AstNode for ShowReturn {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::SHOW_RETURN
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(ShowReturn(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ShowReturn {
    pub fn star_token(&self) -> Option<SyntaxToken> {
        child_token(&self.0, SyntaxKind::STAR)
    }

    pub fn yield_items(&self) -> AstChildren<YieldItem> {
        children(&self.0)
    }

    pub fn where_expr(&self) -> Option<Expression> {
        self.0
            .children_with_tokens()
            .skip_while(|el| !matches!(el.as_token().map(|t| t.kind()), Some(SyntaxKind::KW_WHERE)))
            .filter_map(|el| el.into_node())
            .find_map(Expression::cast)
    }
}

// ============================================================
// UseClause — USE <graph_name>
// ============================================================

#[derive(Clone, Debug)]
pub struct UseClause(SyntaxNode);

impl AstNode for UseClause {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::USE_CLAUSE
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(UseClause(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl UseClause {
    pub fn schema_name(&self) -> Option<SchemaName> {
        child(&self.0)
    }
}

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
