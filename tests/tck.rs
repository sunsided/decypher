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
//! - `analyze` (with the `hir` feature): queries the TCK expects to compile
//!   without error also pass semantic analysis.
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

struct TckQuery {
    id: String,
    source: String,
    accept: bool,
    /// Whether any TCK scenario using this query expects an error.
    expects_error: bool,
}

fn tck_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tck")
}

fn load_queries() -> Vec<TckQuery> {
    let text = std::fs::read_to_string(tck_dir().join("TCK_SYNTAX.jsonl"))
        .expect("tests/tck/TCK_SYNTAX.jsonl is missing");
    let mut queries = BTreeMap::new();
    let mut errors = BTreeSet::new();
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
                };
                queries.insert(id, query);
            }
            Some("occurrence") if !record["upstream_error"].is_null() => {
                let id = str_field("query_id").expect("occurrence without query_id");
                errors.insert(id.trim_start_matches("query:").to_owned());
            }
            _ => {}
        }
    }
    queries
        .into_iter()
        .map(|(sha, mut q)| {
            q.expects_error = errors.contains(&sha);
            q
        })
        .collect()
}

/// Runs `f`, turning a panic into an error message.
fn guarded<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err("panic".to_owned()))
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

/// Identifier and literal tokens of `source`, with escaped identifiers
/// unescaped so that `` `a` `` and `a` compare equal.
fn significant_tokens(source: &str) -> BTreeMap<String, usize> {
    use decypher::syntax::SyntaxKind;
    let mut counts = BTreeMap::new();
    for token in decypher::parse_cst(source)
        .tree
        .descendants_with_tokens()
        .filter_map(|el| el.into_token())
    {
        let text = match token.kind() {
            SyntaxKind::IDENT => token.text().to_owned(),
            SyntaxKind::ESCAPED_IDENT => token.text().trim_matches('`').replace("``", "`"),
            // The printer normalises number syntax (`0x1F` → `31`, `1e9` →
            // `1000000000.0`), so compare numbers by value.
            SyntaxKind::INTEGER | SyntaxKind::FLOAT => number_value(token.text()),
            // …and may switch quote style, so compare strings by content.
            SyntaxKind::STRING => {
                let t = token.text();
                format!("'{}'", t.get(1..t.len().saturating_sub(1)).unwrap_or(t))
            }
            _ => continue,
        };
        *counts.entry(text).or_insert(0) += 1;
    }
    counts
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
