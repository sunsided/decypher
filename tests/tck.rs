//! Conformance gate against the openCypher 2024.3 TCK.
//!
//! `tests/tck/TCK_SYNTAX.jsonl` is a syntax-only projection of the official
//! openCypher TCK (see `tests/tck/README.md`): every Cypher query from the
//! feature files, marked `accept` or `reject`. For each query this test checks:
//!
//! - `parse`: the parser accepts or rejects it as expected;
//! - `roundtrip`: for accepted queries, `parse → to_cypher → parse → to_cypher`
//!   is idempotent, so printing does not change the query;
//! - `lossless`: every identifier and literal of the query still appears in
//!   the printed query. This catches parts dropped while building the AST,
//!   which the idempotency check cannot see once the first print has lost them;
//! - `sema`: queries the TCK expects to compile without error also pass
//!   `sema::analyze`;
//! - `analyze` (with the `hir` feature): such queries also pass HIR lowering;
//! - `reject`: queries the TCK expects to fail at compile time with one of
//!   the [`SEMANTIC_ERRORS`] that semantic analysis covers fail
//!   `sema::analyze` and, with the `hir` feature, `analyze`.
//!
//! Known failures are listed in `tests/tck/known_failures.txt`. The test fails
//! on any failure not in the list and on any listed entry that now passes, so
//! the list can only shrink. After fixing bugs, regenerate it with
//! `TCK_BLESS=1 cargo test --test tck`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use decypher::ast::ToCypher;

/// Number of hex digits of the query hash used as its id.
const ID_LEN: usize = 16;

/// Compile-time TCK error details that both semantic analyzers detect.
const SEMANTIC_ERRORS: &[&str] = &[
    "UndefinedVariable",
    "InvalidAggregation",
    "NestedAggregation",
    "AmbiguousAggregationExpression",
    "ColumnNameConflict",
    "VariableAlreadyBound",
    "DifferentColumnsInUnion",
    "InvalidClauseComposition",
    "CreatingVarLength",
    "NoSingleRelationshipType",
    "RequiresDirectedRelationship",
    "InvalidDelete",
    "RelationshipUniquenessViolation",
    "NoExpressionAlias",
    "NoVariablesInScope",
    "NonConstantExpression",
    "NegativeIntegerArgument",
    "InvalidParameterUse",
    "VariableTypeConflict",
    "UnexpectedSyntax",
    "InvalidArgumentType",
    "UnknownFunction",
];

struct TckQuery {
    id: String,
    source: String,
    accept: bool,
    /// Whether any TCK scenario using this query expects an error.
    expects_error: bool,
    /// Whether a TCK scenario expects one of the [`SEMANTIC_ERRORS`].
    expects_semantic_error: bool,
}

fn tck_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tck")
}

fn load_queries() -> Vec<TckQuery> {
    let text = std::fs::read_to_string(tck_dir().join("TCK_SYNTAX.jsonl"))
        .expect("tests/tck/TCK_SYNTAX.jsonl is missing");
    let mut queries = BTreeMap::new();
    let mut errors = BTreeSet::new();
    let mut semantic_errors = BTreeSet::new();
    for line in text.lines() {
        let record: serde_json::Value = serde_json::from_str(line).expect("invalid JSONL record");
        let str_field = |name: &str| record[name].as_str().map(str::to_owned);
        match record["record"].as_str() {
            Some("query") => {
                let id = str_field("sha256").expect("query without sha256");
                let query = TckQuery {
                    id: id[..ID_LEN].to_owned(),
                    source: str_field("source").expect("query without source"),
                    accept: str_field("expectation").as_deref() == Some("accept"),
                    expects_error: false,
                    expects_semantic_error: false,
                };
                queries.insert(id, query);
            }
            Some("occurrence") if !record["upstream_error"].is_null() => {
                let id = str_field("query_id").expect("occurrence without query_id");
                let id = id.trim_start_matches("query:").to_owned();
                let error = &record["upstream_error"];
                if error["phase"] == "compile time"
                    && SEMANTIC_ERRORS.contains(&error["detail"].as_str().unwrap_or_default())
                {
                    semantic_errors.insert(id.clone());
                }
                errors.insert(id);
            }
            _ => {}
        }
    }
    queries
        .into_iter()
        .map(|(sha, mut q)| {
            q.expects_error = errors.contains(&sha);
            q.expects_semantic_error = semantic_errors.contains(&sha);
            q
        })
        .collect()
}

/// Runs `f`, turning a panic into an error message.
fn guarded<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string payload>".to_owned());
        Err(format!("panic: {message}"))
    })
}

/// Runs all checks for one query, returning `(check, message)` per failure.
fn check_query(q: &TckQuery) -> Vec<(&'static str, String)> {
    let mut failures = Vec::new();
    let parsed = guarded(|| decypher::parse(q.source.as_str()).map_err(|e| e.kind.to_string()));
    match (&parsed, q.accept) {
        (Ok(_), false) => failures.push(("parse", "accepted, expected a syntax error".into())),
        (Err(e), true) => failures.push(("parse", e.clone())),
        _ => {}
    }
    let Ok(ast) = parsed else {
        return failures;
    };

    let roundtrip = guarded(|| {
        let printed = ast.to_cypher();
        let reparsed = decypher::parse(printed.as_str())
            .map_err(|e| format!("printed query fails to parse: {}", e.kind))?;
        if reparsed.to_cypher() == printed {
            Ok(())
        } else {
            Err(format!("printing changes the query: {printed}"))
        }
    });
    if let Err(e) = roundtrip {
        failures.push(("roundtrip", e));
    }

    if let Err(e) = guarded(|| lossless(&q.source, &ast.to_cypher())) {
        failures.push(("lossless", e));
    }

    if q.accept && !q.expects_error {
        let analyzed = guarded(|| {
            decypher::sema::analyze(&ast).map_err(|d| {
                d.errors
                    .first()
                    .map_or_else(String::new, |e| e.kind.to_string())
            })
        });
        if let Err(e) = analyzed {
            failures.push(("sema", e));
        }
    }

    if q.expects_semantic_error {
        // A strict catalog reports unknown functions (`foo(a)`). The TCK
        // declares its procedures per scenario, which the corpus does not
        // record, so procedure signatures are not checked here.
        let mut catalog = decypher::sema::Catalog::default();
        catalog.set_strict(true);
        match guarded(|| Ok(decypher::sema::analyze_with(&ast, &catalog).is_ok())) {
            Ok(true) => failures.push(("reject", "sema::analyze accepts it".into())),
            Ok(false) => {}
            Err(e) => failures.push(("reject", e)),
        }
        #[cfg(feature = "hir")]
        match guarded(|| {
            let config = decypher::hir::LowerConfig { catalog };
            Ok(decypher::analyze_with_config(q.source.as_str(), &config).is_ok())
        }) {
            Ok(true) => failures.push(("reject", "analyze accepts it".into())),
            Ok(false) => {}
            Err(e) => failures.push(("reject", e)),
        }
    }

    #[cfg(feature = "hir")]
    if q.accept && !q.expects_error {
        let analyzed =
            guarded(|| decypher::analyze(q.source.as_str()).map_err(|e| e.kind.to_string()));
        if let Err(e) = analyzed {
            failures.push(("analyze", e));
        }
    }
    failures
}

/// Identifier and literal tokens of `source`, normalised so that equivalent
/// spellings (`` `a` `` and `a`, `0x1F` and `31`, `'x'` and `"x"`, `NULL` and
/// `null`) compare equal.
fn significant_tokens(source: &str) -> BTreeMap<String, usize> {
    use decypher::syntax::SyntaxKind;
    let mut counts = BTreeMap::new();
    for token in decypher::parse_cst(source)
        .tree
        .descendants_with_tokens()
        .filter_map(|el| el.into_token())
    {
        let text = match token.kind() {
            SyntaxKind::IDENT => literal_word(token.text().to_owned()),
            // A keyword used as a name (`{null: 1}`) prints as an escaped
            // identifier, so names spelling a literal compare like literals.
            SyntaxKind::ESCAPED_IDENT => {
                literal_word(token.text().trim_matches('`').replace("``", "`"))
            }
            // The printer normalises number syntax (`0x1F` → `31`, `1e9` →
            // `1000000000.0`), so compare numbers by value.
            SyntaxKind::INTEGER | SyntaxKind::FLOAT => number_value(token.text()),
            // …and may switch quote style, so compare strings by content.
            SyntaxKind::STRING => {
                let t = token.text();
                format!(
                    "'{}'",
                    unescape(t.get(1..t.len().saturating_sub(1)).unwrap_or(t))
                )
            }
            SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW | SyntaxKind::NULL_KW => {
                token.text().to_ascii_lowercase()
            }
            _ => continue,
        };
        *counts.entry(text).or_insert(0) += 1;
    }
    counts
}

/// Lower-cases names that spell `null`, `true` or `false`, matching how the
/// literal tokens are normalised.
fn literal_word(name: String) -> String {
    let lower = name.to_ascii_lowercase();
    if matches!(lower.as_str(), "null" | "true" | "false") {
        lower
    } else {
        name
    }
}

/// Decodes backslash escapes so that `'it\'s'` and `"it's"` compare equal.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// A canonical rendering of a number literal's value.
fn number_value(text: &str) -> String {
    let t = text.to_ascii_lowercase().replace('_', "");
    let int = |digits: &str, radix| u128::from_str_radix(digits, radix).ok().map(|v| v as f64);
    let value = if let Some(hex) = t.strip_prefix("0x") {
        int(hex, 16)
    } else if let Some(oct) = t.strip_prefix("0o") {
        int(oct, 8)
    } else {
        t.parse::<f64>().ok()
    };
    value.map_or(t, |v| format!("{v:e}"))
}

/// Checks that no identifier or literal of `source` is missing from `printed`.
fn lossless(source: &str, printed: &str) -> Result<(), String> {
    let printed_tokens = significant_tokens(printed);
    let missing: Vec<_> = significant_tokens(source)
        .into_iter()
        .filter(|(token, n)| printed_tokens.get(token).copied().unwrap_or(0) < *n)
        .map(|(token, _)| token)
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("printing drops {}: {printed}", missing.join(", ")))
    }
}

fn one_line(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    match flat.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &flat[..i]),
        None => flat,
    }
}

/// Parses `known_failures.txt` into `(check, id)` pairs.
fn load_known_failures() -> BTreeSet<(String, String)> {
    let text = std::fs::read_to_string(tck_dir().join("known_failures.txt")).unwrap_or_default();
    text.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            let check = parts.next().unwrap_or_default().to_owned();
            let id = parts.next().unwrap_or_default().to_owned();
            (check, id)
        })
        .collect()
}

#[test]
fn tck_conformance() {
    let queries = load_queries();
    assert!(queries.len() > 4000, "TCK corpus looks truncated");

    let mut failures = BTreeMap::new();
    for q in &queries {
        for (check, message) in check_query(q) {
            failures.insert((check.to_owned(), q.id.clone()), (message, &q.source));
        }
    }

    if std::env::var_os("TCK_BLESS").is_some() {
        // Without `hir` no analyze failures are produced, so blessing would
        // erase the analyze baseline.
        if !cfg!(feature = "hir") {
            panic!("TCK_BLESS requires the `hir` feature (drop --no-default-features)");
        }
        let mut out = String::from(
            "# Known openCypher TCK failures; see tests/tck.rs.\n\
             # Format: <check> <query id>  # <error> | <query>\n\
             # Regenerate with: TCK_BLESS=1 cargo test --test tck\n",
        );
        for ((check, id), (message, source)) in &failures {
            let _ = writeln!(
                out,
                "{check} {id}  # {} | {}",
                one_line(message, 80),
                one_line(source, 100)
            );
        }
        std::fs::write(tck_dir().join("known_failures.txt"), out).unwrap();
        return;
    }

    // `analyze` needs the `hir` feature; without it, its known failures
    // can't be checked and must not be reported as stale.
    let known: BTreeSet<_> = load_known_failures()
        .into_iter()
        .filter(|(check, _)| cfg!(feature = "hir") || check != "analyze")
        .collect();
    let unexpected: Vec<_> = failures
        .iter()
        .filter(|(key, _)| !known.contains(*key))
        .collect();
    let stale: Vec<_> = known
        .iter()
        .filter(|key| !failures.contains_key(*key))
        .collect();

    let mut report = String::new();
    for ((check, id), (message, source)) in &unexpected {
        let _ = writeln!(
            report,
            "new {check} failure {id}: {message}\n    {}",
            one_line(source, 160)
        );
    }
    for (check, id) in &stale {
        let _ = writeln!(
            report,
            "now passes, remove from known_failures.txt: {check} {id}"
        );
    }
    assert!(
        unexpected.is_empty() && stale.is_empty(),
        "{} new failure(s), {} stale known failure(s) \
         (run `TCK_BLESS=1 cargo test --test tck` to accept the current state):\n{report}",
        unexpected.len(),
        stale.len()
    );
}
