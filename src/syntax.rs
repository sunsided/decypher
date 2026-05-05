//! Syntactic vocabulary for the openCypher lossless CST.
//!
//! This module defines [`SyntaxKind`], the token and node kind enum used by
//! the rowan CST, and [`CypherLang`], the rowan [`Language`] impl that maps
//! between `SyntaxKind` and rowan's raw `u16` IDs.
//!
//! It also re-exports the [`SyntaxNode`] and [`SyntaxToken`] type aliases
//! used throughout the parser and CST traversal code.
//!
//! # Sub-modules
//!
//! The [`ast`] sub-module provides typed newtype wrappers over raw rowan
//! nodes for all openCypher grammar productions (clauses, expressions,
//! patterns, etc.).

use rowan::Language;

/// All token and internal-node kinds used in the openCypher CST.
///
/// Token kinds represent leaf tokens (whitespace, punctuation, literals,
/// keywords). Internal-node kinds represent non-leaf grammar productions
/// (statements, expressions, patterns, etc.).
///
/// The enum is `#[repr(u16)]` so that values can be stored and compared
/// efficiently, and it implements [`rowan::Language`] through [`CypherLang`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    /* Trivia */
    WHITESPACE = 0,
    COMMENT,

    /* Punctuation */
    L_PAREN,
    R_PAREN,
    L_BRACE,
    R_BRACE,
    L_BRACKET,
    R_BRACKET,
    COMMA,
    DOT,
    DOT_DOT,
    COLON,
    PIPE,
    DOLLAR,
    BACKTICK,
    SEMICOLON,

    /* Operators */
    EQ,
    NE,
    LT,
    GT,
    LE,
    GE,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    PERCENT,
    POW,
    PLUSEQ,
    TILDE_EQ,
    TILDE,
    ARROW_LEFT,
    ARROW_RIGHT,
    DASH,
    BANG,
    AMPERSAND,

    /* Literals */
    INTEGER,
    FLOAT,
    STRING,
    TRUE_KW,
    FALSE_KW,
    NULL_KW,
    IDENT,
    ESCAPED_IDENT,

    /* Keywords (non-punctuation operators) */
    KW_ACCESS,
    KW_ADD,
    KW_ALL,
    KW_AND,
    KW_ANY,
    KW_AS,
    KW_ASC,
    KW_ASCENDING,
    KW_BREAK,
    KW_BY,
    KW_CALL,
    KW_CASE,
    KW_CONTAINS,
    KW_CONTINUE,
    KW_CONSTRAINT,
    KW_CONSTRAINTS,
    KW_CREATE,
    KW_DATABASE,
    KW_DATABASES,
    KW_DELETE,
    KW_DESC,
    KW_DESCENDING,
    KW_DETACH,
    KW_DISTINCT,
    KW_DO,
    KW_DROP,
    KW_ELSE,
    KW_END,
    KW_ENDS,
    KW_ERROR,
    KW_EXISTS,
    KW_EXTRACT,
    KW_FAIL,
    KW_FILTER,
    KW_FOR,
    KW_FOREACH,
    KW_EACH,
    KW_FUNCTIONS,
    KW_FULLTEXT,
    KW_IF,
    KW_IN,
    KW_INDEX,
    KW_INDEXES,
    KW_IS,
    KW_KEY,
    KW_LIMIT,
    KW_LOOKUP,
    KW_MANDATORY,
    KW_MATCH,
    KW_MERGE,
    KW_NODE,
    KW_NONE,
    KW_NOT,
    KW_OF,
    KW_ON,
    KW_OPTIONAL,
    KW_OPTIONS,
    KW_OR,
    KW_ORDER,
    KW_POINT,
    KW_PROCEDURES,
    KW_PROPERTY,
    KW_RANGE,
    KW_REDUCE,
    KW_REMOVE,
    KW_REQUIRE,
    KW_RETURN,
    KW_ROWS,
    KW_SCALAR,
    KW_SET,
    KW_SHOW,
    KW_SINGLE,
    KW_SKIP,
    KW_STARTS,
    KW_TEXT,
    KW_THEN,
    KW_TRANSACTIONS,
    KW_TYPE,
    KW_TYPES,
    KW_UNION,
    KW_UNIQUE,
    KW_UNWIND,
    KW_USE,
    KW_WHEN,
    KW_WHERE,
    KW_WITH,
    KW_XOR,
    KW_YIELD,
    KW_COUNT,
    KW_CALL_SUBQUERY,
    KW_IN_TRANSACTIONS,
    KW_CONCURRENTLY,
    KW_GRAPH,
    KW_HEADERS,
    KW_FROM,
    KW_LOAD,
    KW_CSV,
    KW_FINISH,
    KW_FIELDTERMINATOR,

    /* Composite nodes (CST non-terminals) */
    SOURCE_FILE,
    STATEMENT,
    QUERY,
    REGULAR_QUERY,
    UNION,
    SINGLE_QUERY,
    SINGLE_PART_QUERY,
    MULTI_PART_QUERY,
    READING_CLAUSE,
    UPDATING_CLAUSE,
    MATCH_CLAUSE,
    UNWIND_CLAUSE,
    MERGE_CLAUSE,
    MERGE_ACTION,
    CREATE_CLAUSE,
    SET_CLAUSE,
    SET_ITEM,
    DELETE_CLAUSE,
    REMOVE_CLAUSE,
    REMOVE_ITEM,
    FOREACH_CLAUSE,
    CALL_SUBQUERY_CLAUSE,
    IN_TRANSACTIONS,
    IN_QUERY_CALL,
    STANDALONE_CALL,
    YIELD_ITEMS,
    YIELD_ITEM,
    WITH_CLAUSE,
    RETURN_CLAUSE,
    LOAD_CSV_CLAUSE,
    FINISH_CLAUSE,
    PROJECTION_BODY,
    PROJECTION_ITEMS,
    PROJECTION_ITEM,
    ORDER_BY,
    SORT_ITEM,
    SKIP_CLAUSE,
    LIMIT_CLAUSE,
    WHERE_CLAUSE,
    PATTERN,
    PATTERN_PART,
    ANONYMOUS_PATTERN_PART,
    PATTERN_ELEMENT,
    NODE_PATTERN,
    PATTERN_ELEMENT_CHAIN,
    RELATIONSHIP_PATTERN,
    RELATIONSHIP_DETAIL,
    RELATIONSHIP_QUANTIFIER,
    QUANTIFIED_PATH_PATTERN,
    PROPERTIES,
    RELATIONSHIP_TYPES,
    NODE_LABELS,
    LABEL_EXPRESSION,
    LABEL_OR,
    LABEL_AND,
    LABEL_NOT,
    LABEL_PAREN,
    LABEL_ATOM,
    NODE_LABEL,
    DYNAMIC_LABEL,
    RANGE_LITERAL,
    LABEL_NAME,
    REL_TYPE_NAME,
    DYNAMIC_REL_TYPE,
    EXPRESSION,
    OR_EXPR,
    XOR_EXPR,
    AND_EXPR,
    NOT_EXPR,
    COMPARISON_EXPR,
    ADD_SUB_EXPR,
    MUL_DIV_MOD_EXPR,
    POWER_EXPR,
    UNARY_ADD_SUB_EXPR,
    STRING_LIST_NULL_EXPR,
    LIST_OP_EXPR,
    STRING_OP_EXPR,
    NULL_OP_EXPR,
    PROPERTY_OR_LABELS_EXPR,
    ATOM,
    LITERAL,
    NUMBER_LITERAL,
    STRING_LITERAL,
    BOOLEAN_LITERAL,
    LIST_LITERAL,
    MAP_LITERAL,
    MAP_ENTRY,
    PARAMETER,
    VARIABLE,
    SYMBOLIC_NAME,
    FUNCTION_INVOCATION,
    FUNCTION_NAME,
    NAMESPACE,
    EXPLICIT_PROCEDURE_INVOCATION,
    IMPLICIT_PROCEDURE_INVOCATION,
    PROCEDURE_RESULT_FIELD,
    PROCEDURE_NAME,
    CASE_EXPR,
    CASE_ALTERNATIVE,
    LIST_COMPREHENSION,
    PATTERN_COMPREHENSION,
    FILTER_EXPRESSION,
    ID_IN_COLL,
    PARENTHESIZED_EXPR,
    RELATIONSHIPS_PATTERN,
    EXISTS_SUBQUERY,
    COUNT_SUBQUERY,
    COLLECT_SUBQUERY,
    MAP_PROJECTION,
    MAP_PROJECTION_ITEM,
    PROPERTY_LOOKUP,
    PROPERTY_EXPRESSION,
    PROPERTY_KEY_NAME,
    SCHEMA_NAME,
    CREATE_INDEX,
    DROP_INDEX,
    CREATE_CONSTRAINT,
    DROP_CONSTRAINT,
    SCHEMA_COMMAND,
    INDEX_KIND,
    CONSTRAINT_KIND,
    OPTIONS_CLAUSE,
    SHOW_CLAUSE,
    SHOW_KIND,
    SHOW_RETURN,
    USE_CLAUSE,
    PARTIAL_COMPARISON,
    ESCAPED_CHAR,

    /* Error node */
    ERROR,
}

impl SyntaxKind {
    /// Convert a raw `u16` token id into a [`SyntaxKind`].
    ///
    /// Returns [`SyntaxKind::ERROR`] for values that are out of range.
    pub fn from(raw: u16) -> Self {
        if raw <= Self::ERROR as u16 {
            unsafe { std::mem::transmute::<u16, SyntaxKind>(raw) }
        } else {
            SyntaxKind::ERROR
        }
    }

    /// Convert this kind into its raw `u16` discriminant.
    pub fn into_u16(self) -> u16 {
        self as u16
    }
}

/// The rowan [`Language`] marker for openCypher.
///
/// Bridges between [`SyntaxKind`] and the raw `u16` IDs stored in
/// rowan's generic CST.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CypherLang;

impl Language for CypherLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

/// A concrete syntax-tree node typed with [`CypherLang`].
pub type SyntaxNode = rowan::SyntaxNode<CypherLang>;
/// A syntax token typed with [`CypherLang`].
pub type SyntaxToken = rowan::SyntaxToken<CypherLang>;
/// A syntax element (either a node or a token) typed with [`CypherLang`].
pub type SyntaxElement = rowan::SyntaxElement<CypherLang>;
/// An iterator over the children of a [`SyntaxNode`].
pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<CypherLang>;
/// An iterator over element children (nodes and tokens) of a [`SyntaxNode`].
pub type SyntaxElementChildren = rowan::SyntaxElementChildren<CypherLang>;
/// A pre-order traversal iterator over [`SyntaxNode`]s.
pub type Preorder = rowan::api::Preorder<CypherLang>;
/// A pre-order traversal iterator that also yields [`SyntaxToken`]s.
pub type PreorderWithTokens = rowan::api::PreorderWithTokens<CypherLang>;

pub mod ast;
