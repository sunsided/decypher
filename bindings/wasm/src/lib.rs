//! WebAssembly bindings for the [`decypher`] Cypher parser.
//!
//! The exported functions never throw for invalid Cypher: every call returns a
//! plain JavaScript object with an `ok` flag, so callers can branch on it
//! instead of wrapping each call in `try`/`catch`. Only programming errors (for
//! example passing a non-string where a string is expected) surface as
//! exceptions.
//!
//! All spans inside the AST and CST are **byte offsets into the UTF-8 encoded
//! source**, matching the Rust API. Diagnostics additionally carry UTF-16
//! offsets (what `String.prototype.slice` expects) and 1-based line/column
//! positions so editors can use them directly.

use decypher::CypherError;
use decypher::ast::ToCypher;
use decypher::syntax::{SyntaxElement, SyntaxNode};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/** A half-open `[start, end)` range of byte offsets into the UTF-8 source. */
export interface Span {
  start: number;
  end: number;
}

export type NoteLevel = "info" | "warning" | "help";

export interface Note {
  level: NoteLevel;
  message: string;
  span: Span;
}

export interface Diagnostic {
  /** One-line description of the problem. */
  message: string;
  /** Byte offsets into the UTF-8 source. */
  span: Span;
  /** UTF-16 offsets, usable with `source.slice(start, end)`. */
  range: Span;
  /** 1-based line of `span.start`. */
  line: number;
  /** 1-based column (in characters) of `span.start`. */
  column: number;
  notes: Note[];
  /** Multi-line, human-readable report including a source excerpt. */
  rendered: string;
}

export type Result<T> =
  | ({ ok: true } & T)
  | { ok: false; errors: Diagnostic[] };

export interface ParseOutput {
  /**
   * The typed AST, serialised from `decypher::ast::Query`. Enum variants are
   * externally tagged, e.g. `{ "Match": { ... } }`.
   */
  ast: Query;
}

/** Root of the typed AST (`decypher::ast::Query`). */
export interface Query {
  statements: unknown[];
  span: Span;
}

export interface FormatOutput {
  cypher: string;
}

export interface CstNode {
  kind: string;
  span: Span;
  children: CstElement[];
}

export interface CstToken {
  kind: string;
  span: Span;
  text: string;
}

export type CstElement = CstNode | CstToken;

export interface CstOutput {
  /** The lossless syntax tree; present even when `errors` is non-empty. */
  tree: CstNode;
  errors: Diagnostic[];
}
"#;

#[derive(Serialize)]
struct Span {
    start: usize,
    end: usize,
}

impl From<decypher::Span> for Span {
    fn from(span: decypher::Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Serialize)]
struct Note {
    level: String,
    message: String,
    span: Span,
}

#[derive(Serialize)]
struct Diagnostic {
    message: String,
    span: Span,
    range: Span,
    line: usize,
    column: usize,
    notes: Vec<Note>,
    rendered: String,
}

impl Diagnostic {
    fn new(error: &CypherError, source: &str) -> Self {
        let span = error.span();
        let (line, column) = span.line_col(source);
        Self {
            message: error.kind().to_string(),
            range: Span {
                start: utf16_offset(source, span.start),
                end: utf16_offset(source, span.end),
            },
            span: span.into(),
            line,
            column,
            notes: error
                .notes()
                .iter()
                .map(|note| Note {
                    level: note.level.to_string(),
                    message: note.message.to_string(),
                    span: note.span.into(),
                })
                .collect(),
            rendered: error.render(source),
        }
    }
}

/// Convert a UTF-8 byte offset into a UTF-16 code-unit offset.
fn utf16_offset(source: &str, byte_offset: usize) -> usize {
    let byte_offset = byte_offset.min(source.len());
    let mut boundary = byte_offset;
    while !source.is_char_boundary(boundary) {
        boundary -= 1;
    }
    source[..boundary].encode_utf16().count()
}

fn diagnostics<'a>(
    errors: impl IntoIterator<Item = &'a CypherError>,
    source: &str,
) -> Vec<Diagnostic> {
    errors
        .into_iter()
        .map(|error| Diagnostic::new(error, source))
        .collect()
}

#[derive(Serialize)]
#[serde(untagged)]
enum Outcome<T> {
    Ok {
        ok: bool,
        #[serde(flatten)]
        value: T,
    },
    Err {
        ok: bool,
        errors: Vec<Diagnostic>,
    },
}

impl<T> Outcome<T> {
    fn ok(value: T) -> Self {
        Outcome::Ok { ok: true, value }
    }

    fn err(errors: Vec<Diagnostic>) -> Self {
        Outcome::Err { ok: false, errors }
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    Ok(value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?)
}

fn parse_recovering(source: &str) -> Result<decypher::Query, Vec<Diagnostic>> {
    match decypher::parse_all(source) {
        (Some(query), errors) if errors.is_empty() => Ok(query),
        (_, errors) => Err(diagnostics(errors.iter(), source)),
    }
}

/// Parse Cypher into the typed AST.
///
/// Uses error recovery, so a failed result lists every syntax error found
/// rather than only the first one.
#[wasm_bindgen(unchecked_return_type = "Result<ParseOutput>")]
pub fn parse(source: &str) -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct ParseOutput {
        ast: decypher::Query,
    }

    let outcome = match parse_recovering(source) {
        Ok(ast) => Outcome::ok(ParseOutput { ast }),
        Err(errors) => Outcome::err(errors),
    };
    to_js(&outcome)
}

/// Check a query for syntax errors, unresolved variables (including ones
/// dropped from scope by `WITH`) and invalid aggregations without returning
/// the tree.
#[wasm_bindgen(unchecked_return_type = "Result<{}>")]
pub fn validate(source: &str) -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct Empty {}

    let query = match parse_recovering(source) {
        Ok(query) => query,
        Err(errors) => return to_js(&Outcome::<Empty>::err(errors)),
    };

    let outcome = match decypher::hir::lower::lower(&query, &Default::default()) {
        Ok(_) => Outcome::ok(Empty {}),
        Err(errors) => Outcome::err(diagnostics(errors.iter(), source)),
    };
    to_js(&outcome)
}

/// Re-emit a query as canonical single-line Cypher (uppercase keywords,
/// normalised whitespace). Comments are not preserved.
#[wasm_bindgen(unchecked_return_type = "Result<FormatOutput>")]
pub fn format(source: &str) -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct FormatOutput {
        cypher: String,
    }

    let outcome = match parse_recovering(source) {
        Ok(query) => Outcome::ok(FormatOutput {
            cypher: query.to_cypher(),
        }),
        Err(errors) => Outcome::err(errors),
    };
    to_js(&outcome)
}

#[derive(Serialize)]
#[serde(untagged)]
enum CstElement {
    Node {
        kind: String,
        span: Span,
        children: Vec<CstElement>,
    },
    Token {
        kind: String,
        span: Span,
        text: String,
    },
}

fn cst_node(node: &SyntaxNode) -> CstElement {
    let range = node.text_range();
    CstElement::Node {
        kind: format!("{:?}", node.kind()),
        span: Span {
            start: range.start().into(),
            end: range.end().into(),
        },
        children: node
            .children_with_tokens()
            .map(|child| match child {
                SyntaxElement::Node(node) => cst_node(&node),
                SyntaxElement::Token(token) => {
                    let range = token.text_range();
                    CstElement::Token {
                        kind: format!("{:?}", token.kind()),
                        span: Span {
                            start: range.start().into(),
                            end: range.end().into(),
                        },
                        text: token.text().to_string(),
                    }
                }
            })
            .collect(),
    }
}

/// Parse Cypher into the lossless concrete syntax tree (every token, including
/// whitespace and comments). Always returns a tree; `errors` lists the syntax
/// errors encountered along the way.
#[wasm_bindgen(unchecked_return_type = "CstOutput")]
pub fn cst(source: &str) -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct CstOutput {
        tree: CstElement,
        errors: Vec<Diagnostic>,
    }

    let parse = decypher::parse_cst(source);
    to_js(&CstOutput {
        tree: cst_node(&parse.tree),
        errors: diagnostics(parse.errors.iter(), source),
    })
}

/// The version of these bindings (kept in lockstep with the `decypher` crate).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::utf16_offset;

    #[test]
    fn utf16_offsets_account_for_multibyte_characters() {
        let source = "RETURN '😀' AS x";
        // The emoji is 4 bytes in UTF-8 but 2 code units in UTF-16.
        let after_emoji = source.find("' AS").unwrap();
        assert_eq!(after_emoji, 12);
        assert_eq!(utf16_offset(source, after_emoji), 10);
    }

    #[test]
    fn utf16_offsets_clamp_and_snap_to_char_boundaries() {
        let source = "é";
        assert_eq!(utf16_offset(source, 1), 0);
        assert_eq!(utf16_offset(source, 99), 1);
    }
}
