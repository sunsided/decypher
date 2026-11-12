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
use decypher::hir::arena::{Id, Interner};
use decypher::hir::{
    BindingId, BindingKind, ExprKind, GraphPattern, HirQuery, LabelExpr, Operation,
    RelationshipDirection, RelationshipLength,
};
use decypher::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
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

/** A variable and every place it occurs, as UTF-16 ranges. */
export interface Variable {
  name: string;
  /** How it was bound, e.g. `"PatternBound"`, `"WithAlias"`, `"UnwindBound"`. */
  kind: string;
  definition: Span;
  /** References after the definition, including ones in nested subqueries. */
  uses: Span[];
}

export interface GraphNode {
  variable: string | null;
  /** Label names, or one entry with the label expression as text (`"A|B"`, `"!A"`, `"%"`). */
  labels: string[];
}

export interface GraphRel {
  variable: string | null;
  /** Type names, or one entry with the type expression as text (`"!R"`). */
  types: string[];
  /** `"right"` for `-->`, `"left"` for `<--`, `"none"` for `--`, `"both"` for `<-->`. */
  direction: "right" | "left" | "none" | "both";
  /** Index into the pattern's `nodes`. */
  from: number;
  /** Index into the pattern's `nodes`. */
  to: number;
  variableLength: boolean;
}

export interface Pattern {
  clause: "MATCH" | "OPTIONAL MATCH" | "CREATE" | "MERGE";
  nodes: GraphNode[];
  relationships: GraphRel[];
}

export interface AnalyzeOutput {
  /** Ordered by definition. */
  variables: Variable[];
  patterns: Pattern[];
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

#[derive(Serialize)]
struct Variable {
    name: String,
    kind: String,
    definition: Span,
    uses: Vec<Span>,
}

#[derive(Serialize)]
struct GraphNode {
    variable: Option<String>,
    labels: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphRel {
    variable: Option<String>,
    types: Vec<String>,
    direction: &'static str,
    from: usize,
    to: usize,
    variable_length: bool,
}

#[derive(Serialize)]
struct Pattern {
    clause: &'static str,
    nodes: Vec<GraphNode>,
    relationships: Vec<GraphRel>,
}

#[derive(Serialize)]
struct AnalyzeOutput {
    variables: Vec<Variable>,
    patterns: Vec<Pattern>,
}

/// Collects variables and graph patterns from a lowered query and its nested
/// subqueries. Spans are kept as byte offsets until [`Collector::finish`].
struct Collector {
    /// `(name, kind, definition, uses)`; one entry per distinct variable.
    variables: Vec<(String, String, decypher::Span, Vec<decypher::Span>)>,
    patterns: Vec<Pattern>,
}

impl Collector {
    fn variable(&mut self, name: &str, kind: BindingKind, span: decypher::Span) -> usize {
        // Subqueries re-bind imported variables with the outer definition span,
        // so `(name, span)` identifies one variable across nested queries.
        if let Some(index) = self
            .variables
            .iter()
            .position(|(n, _, def, _)| n == name && *def == span)
        {
            return index;
        }
        self.variables
            .push((name.to_string(), format!("{kind:?}"), span, Vec::new()));
        self.variables.len() - 1
    }

    fn query(&mut self, query: &HirQuery) {
        let arenas = &query.arenas;
        let mut by_binding = std::collections::HashMap::new();
        for (id, binding) in arenas.bindings.iter() {
            let index = self.variable(&binding.name, binding.kind, binding.introduced_at);
            by_binding.insert(id, index);
        }
        for (_, expr) in arenas.expressions.iter() {
            match &expr.kind {
                ExprKind::Binding(id) => {
                    if let Some(&index) = by_binding.get(id) {
                        self.variables[index].3.push(expr.span);
                    }
                }
                ExprKind::ExistsSubquery(sub) => self.query(&sub.query),
                ExprKind::CountSubquery(sub) => self.query(&sub.query),
                ExprKind::CollectSubquery(sub) => self.query(&sub.query),
                _ => {}
            }
        }
        for part in &query.parts {
            self.operations(&part.operations, query);
        }
    }

    fn operations(&mut self, operations: &[Operation], query: &HirQuery) {
        for operation in operations {
            match operation {
                Operation::Match(op) => self.pattern("MATCH", &op.pattern, query),
                Operation::OptionalMatch(op) => self.pattern("OPTIONAL MATCH", &op.pattern, query),
                Operation::Create(op) => self.pattern("CREATE", &op.pattern, query),
                Operation::Merge(op) => self.pattern("MERGE", &op.pattern, query),
                Operation::Foreach(op) => self.operations(&op.operations, query),
                Operation::CallSubquery(op) => self.query(&op.query),
                Operation::Union(op) => op.branches.iter().for_each(|branch| self.query(branch)),
                _ => {}
            }
        }
    }

    fn pattern(&mut self, clause: &'static str, pattern: &GraphPattern, query: &HirQuery) {
        let arenas = &query.arenas;
        let name = |id: Option<BindingId>| id.map(|id| arenas.bindings.get(id).name.clone());
        self.patterns.push(Pattern {
            clause,
            nodes: pattern
                .nodes
                .iter()
                .map(|node| GraphNode {
                    variable: name(node.binding),
                    labels: match &node.label_expression {
                        Some(expr) => vec![label_expression_text(expr, &arenas.labels)],
                        None => node
                            .labels
                            .iter()
                            .filter_map(|&id| arenas.labels.name_of(id).map(str::to_string))
                            .collect(),
                    },
                })
                .collect(),
            relationships: pattern
                .relationships
                .iter()
                .map(|rel| GraphRel {
                    variable: name(rel.binding),
                    types: match &rel.type_expression {
                        Some(expr) => vec![label_expression_text(expr, &arenas.relationship_types)],
                        None => rel
                            .types
                            .iter()
                            .filter_map(|&id| {
                                arenas.relationship_types.name_of(id).map(str::to_string)
                            })
                            .collect(),
                    },
                    direction: match rel.direction {
                        RelationshipDirection::LeftToRight => "right",
                        RelationshipDirection::RightToLeft => "left",
                        RelationshipDirection::Undirected => "none",
                        RelationshipDirection::Both => "both",
                    },
                    from: rel.left,
                    to: rel.right,
                    variable_length: matches!(rel.length, RelationshipLength::Variable { .. }),
                })
                .collect(),
        });
    }

    fn finish(self, source: &str) -> AnalyzeOutput {
        // `WITH b` and `RETURN b` re-bind `b` under the same name, with the
        // new binding defined at the old one's use. Fold such chains into one
        // variable so a rename covers every occurrence.
        let mut found = self.variables;
        // An unaliased `RETURN n.name` or `count(*)` binds a column named
        // after the expression; it is no variable anyone wrote or can rename.
        found.retain(|(name, _, definition, _)| {
            let text = &source[definition.start..definition.end];
            is_name(text) && unquote(text) == *name
        });
        found.sort_by_key(|(_, _, definition, _)| definition.start);
        let mut variables: Vec<(String, String, decypher::Span, Vec<decypher::Span>)> = Vec::new();
        let mut owner: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();
        for (name, kind, definition, uses) in found {
            let index = match owner.get(&(definition.start, definition.end)) {
                Some(&index) if variables[index].0 == name => index,
                _ => {
                    variables.push((name, kind, definition, Vec::new()));
                    variables.len() - 1
                }
            };
            for span in uses {
                owner.insert((span.start, span.end), index);
                variables[index].3.push(span);
            }
        }

        // The HIR keeps no span for a pattern that refers to an already bound
        // variable (`NOT (b)-->(a)`, `EXISTS { MATCH (n)-->() }`). Take those
        // from the syntax tree and attach each to the closest earlier
        // definition of the same name.
        let known: std::collections::HashSet<_> = variables
            .iter()
            .flat_map(|(_, _, definition, uses)| uses.iter().chain([definition]))
            .map(|span| (span.start, span.end))
            .collect();
        for (name, span) in variable_tokens(source) {
            if known.contains(&(span.start, span.end)) {
                continue;
            }
            if let Some(variable) = variables
                .iter_mut()
                .filter(|(n, _, definition, _)| *n == name && definition.start <= span.start)
                .max_by_key(|(_, _, definition, _)| definition.start)
            {
                variable.3.push(span);
            }
        }

        let range = |span: decypher::Span| Span {
            start: utf16_offset(source, span.start),
            end: utf16_offset(source, span.end),
        };
        AnalyzeOutput {
            variables: variables
                .into_iter()
                .map(|(name, kind, definition, mut uses)| {
                    uses.sort_by_key(|span| span.start);
                    uses.dedup();
                    Variable {
                        name,
                        kind,
                        definition: range(definition),
                        uses: uses.into_iter().map(range).collect(),
                    }
                })
                .collect(),
            patterns: self.patterns,
        }
    }
}

/// A label or relationship-type expression as text (`A|B`, `!A`, `%`,
/// `$(…)`), parenthesising `|` inside `&` and binary operands of `!`.
fn label_expression_text(expr: &LabelExpr, names: &Interner<Id>) -> String {
    let operand = |inner: &LabelExpr, parenthesise: bool| {
        let text = label_expression_text(inner, names);
        if parenthesise {
            format!("({text})")
        } else {
            text
        }
    };
    match expr {
        LabelExpr::Name(id) => names.name_of(*id).unwrap_or("?").to_string(),
        LabelExpr::Dynamic(_) => "$(…)".to_string(),
        LabelExpr::Wildcard => "%".to_string(),
        LabelExpr::Or(a, b) => format!("{}|{}", operand(a, false), operand(b, false)),
        LabelExpr::And(a, b) => {
            let or = |e: &LabelExpr| matches!(e, LabelExpr::Or(..));
            format!("{}&{}", operand(a, or(a)), operand(b, or(b)))
        }
        LabelExpr::Not(inner) => {
            let binary = matches!(**inner, LabelExpr::Or(..) | LabelExpr::And(..));
            format!("!{}", operand(inner, binary))
        }
    }
}

/// Every `VARIABLE` node in the syntax tree as `(name, identifier span)`.
fn variable_tokens(source: &str) -> Vec<(String, decypher::Span)> {
    decypher::parse_cst(source)
        .tree
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::VARIABLE)
        .filter_map(|node| {
            let token = node
                .descendants_with_tokens()
                .filter_map(|element| element.into_token())
                // Keyword-named variables (`from`, `count`) lex as keywords.
                .find(|token| {
                    !matches!(token.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT)
                })?;
            let range = token.text_range();
            Some((
                unquote(token.text()),
                decypher::Span::new(range.start().into(), range.end().into()),
            ))
        })
        .collect()
}

/// Whether `text` is a single name: an identifier or a backticked name.
fn is_name(text: &str) -> bool {
    let quoted = text.len() >= 2 && text.starts_with('`') && text.ends_with('`');
    let plain = text
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_alphanumeric() || c == '_');
    quoted || plain
}

/// A name as written in the source, without its backticks if it has them.
fn unquote(text: &str) -> String {
    match text.strip_prefix('`').and_then(|t| t.strip_suffix('`')) {
        Some(inner) => inner.replace("``", "`"),
        None => text.to_string(),
    }
}

fn analyze_query(source: &str) -> Outcome<AnalyzeOutput> {
    let query = match parse_recovering(source) {
        Ok(query) => query,
        Err(errors) => return Outcome::err(errors),
    };
    match decypher::hir::lower::lower(&query, &Default::default()) {
        Ok(hir) => {
            let mut collector = Collector {
                variables: Vec::new(),
                patterns: Vec::new(),
            };
            collector.query(&hir);
            Outcome::ok(collector.finish(source))
        }
        Err(errors) => Outcome::err(diagnostics(errors.iter(), source)),
    }
}

/// Resolve a query's variables (where each is defined and every place it is
/// used, as UTF-16 ranges) and flatten its graph patterns into nodes and
/// relationships. Fails like [`validate`] when the query has errors.
#[wasm_bindgen(unchecked_return_type = "Result<AnalyzeOutput>")]
pub fn analyze(source: &str) -> Result<JsValue, JsError> {
    to_js(&analyze_query(source))
}

/// The version of these bindings (kept in lockstep with the `decypher` crate).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::{AnalyzeOutput, Outcome, analyze_query, utf16_offset};

    fn analyzed(source: &str) -> AnalyzeOutput {
        match analyze_query(source) {
            Outcome::Ok { value, .. } => value,
            Outcome::Err { errors, .. } => panic!("{}", errors[0].rendered),
        }
    }

    /// Every occurrence of `name` (definition first), as source slices.
    fn occurrences<'a>(
        source: &'a str,
        output: &AnalyzeOutput,
        name: &str,
    ) -> Vec<(usize, &'a str)> {
        let variable = output.variables.iter().find(|v| v.name == name).unwrap();
        // The spans are UTF-16 offsets; map them to bytes to slice `source`.
        let byte = |utf16: usize| {
            let mut units = 0;
            source
                .char_indices()
                .find(|(_, c)| {
                    let before = units;
                    units += c.len_utf16();
                    before >= utf16
                })
                .map_or(source.len(), |(i, _)| i)
        };
        std::iter::once(&variable.definition)
            .chain(&variable.uses)
            .map(|span| (span.start, &source[byte(span.start)..byte(span.end)]))
            .collect()
    }

    #[test]
    fn variables_follow_with_and_pattern_references() {
        let source = "MATCH (a)-->(b) WHERE NOT (b)-->(a) WITH b, count(*) AS n RETURN b.x AS x, n";
        let output = analyzed(source);
        let names: Vec<_> = output.variables.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "n", "x"]);
        assert_eq!(occurrences(source, &output, "a"), [(7, "a"), (33, "a")]);
        // `WITH b` re-binds `b`; it stays one variable across the boundary.
        assert_eq!(
            occurrences(source, &output, "b"),
            [(13, "b"), (27, "b"), (41, "b"), (65, "b")]
        );
        assert_eq!(occurrences(source, &output, "n"), [(56, "n"), (75, "n")]);
    }

    #[test]
    fn subquery_uses_belong_to_the_outer_variable() {
        let source = "MATCH (n) WHERE EXISTS { MATCH (n)-->(m) } CALL { WITH n RETURN n.x AS y } RETURN n, y";
        let output = analyzed(source);
        assert_eq!(
            occurrences(source, &output, "n"),
            [(7, "n"), (32, "n"), (55, "n"), (64, "n"), (82, "n")]
        );
        assert_eq!(occurrences(source, &output, "y"), [(71, "y"), (85, "y")]);
    }

    #[test]
    fn pattern_subqueries_resolve_imported_variables() {
        let source = "MATCH (m) WHERE EXISTS { (m)-->(x) } RETURN COUNT { (m)<--() } AS n";
        let output = analyzed(source);
        assert_eq!(
            occurrences(source, &output, "m"),
            [(7, "m"), (26, "m"), (53, "m")]
        );
        assert_eq!(output.patterns.len(), 3);
    }

    #[test]
    fn keyword_named_variables_are_found_in_patterns() {
        let source = "MATCH (from)-->(to) MATCH (from)-->(x) RETURN x";
        let output = analyzed(source);
        assert_eq!(
            occurrences(source, &output, "from"),
            [(7, "from"), (27, "from")]
        );
    }

    #[test]
    fn patterns_list_nodes_and_relationships() {
        let output = analyzed("OPTIONAL MATCH (a:Person)<-[r:KNOWS*2]-(:Robot) RETURN a, r");
        let pattern = &output.patterns[0];
        assert_eq!(pattern.clause, "OPTIONAL MATCH");
        assert_eq!(pattern.nodes[0].labels, ["Person"]);
        assert_eq!(pattern.nodes[1].variable, None);
        let rel = &pattern.relationships[0];
        assert_eq!(
            (rel.variable.as_deref(), rel.direction),
            (Some("r"), "left")
        );
        assert_eq!((rel.from, rel.to, rel.variable_length), (0, 1, true));
        assert_eq!(rel.types, ["KNOWS"]);
    }

    #[test]
    fn variable_ranges_are_utf16() {
        let source = "WITH '😀' AS s RETURN s";
        let output = analyzed(source);
        let s = &output.variables[0];
        let utf16: Vec<u16> = source.encode_utf16().collect();
        for span in std::iter::once(&s.definition).chain(&s.uses) {
            assert_eq!(
                String::from_utf16(&utf16[span.start..span.end]).unwrap(),
                "s"
            );
        }
    }

    #[test]
    fn unaliased_columns_are_not_variables() {
        let source = "MATCH (`my n`) RETURN `my n`.name, count(*), `my n`";
        let output = analyzed(source);
        let names: Vec<_> = output.variables.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["my n"]);
        assert_eq!(
            occurrences(source, &output, "my n"),
            [(7, "`my n`"), (22, "`my n`"), (45, "`my n`")]
        );
    }

    #[test]
    fn label_and_type_expressions_are_kept() {
        let output = analyzed("MATCH (n:A|B)-[:!R]->(m:(A|B)&!C), (:%) RETURN n, m");
        let pattern = &output.patterns[0];
        let labels: Vec<_> = pattern.nodes.iter().map(|n| n.labels.clone()).collect();
        assert_eq!(labels, [vec!["A|B"], vec!["(A|B)&!C"], vec!["%"]]);
        assert_eq!(pattern.relationships[0].types, ["!R"]);
    }

    #[test]
    fn occurrences_after_multibyte_text() {
        let source = "WITH '😀' AS s RETURN s";
        let output = analyzed(source);
        // `s` sits after a 4-byte, 2-unit emoji: byte and UTF-16 offsets differ.
        assert_eq!(occurrences(source, &output, "s"), [(13, "s"), (22, "s")]);
    }

    #[test]
    fn analyze_reports_errors() {
        assert!(matches!(
            analyze_query("MATCH (n) RETURN m"),
            Outcome::Err { .. }
        ));
        assert!(matches!(
            analyze_query("MATCH (n RETURN n"),
            Outcome::Err { .. }
        ));
    }

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
