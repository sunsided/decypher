//! Every expression's span covers the whole expression: it is non-empty and
//! contains the spans of its sub-expressions.

use decypher::ast::expr::Expression;
use decypher::ast::visit::{Visit, walk_expression};

struct Check<'s> {
    source: &'s str,
    parents: Vec<(usize, usize)>,
    checked: usize,
}

impl<'a> Visit<'a> for Check<'_> {
    fn visit_expression(&mut self, e: &'a Expression) {
        let s = e.span();
        assert!(
            s.start < s.end && s.end <= self.source.len(),
            "empty or out-of-range span {s:?} for {e:?}"
        );
        if let Some(&(start, end)) = self.parents.last() {
            assert!(
                start <= s.start && s.end <= end,
                "`{}` ({s:?}) lies outside its parent `{}`",
                &self.source[s.start..s.end],
                &self.source[start..end],
            );
        }
        self.checked += 1;
        self.parents.push((s.start, s.end));
        walk_expression(self, e);
        self.parents.pop();
    }
}

#[test]
fn expression_spans_cover_their_sub_expressions() {
    // Suite 5 holds queries that must fail to parse; suite 2 is schema
    // commands without expressions.
    let mut checked = 0;
    for suite in 1..=4 {
        let path = format!("{}/tests/suite-{suite}.cypher", env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(&path).unwrap();
        let query = decypher::parse(source.as_str()).unwrap_or_else(|e| panic!("{path}: {e}"));
        let mut check = Check {
            source: &source,
            parents: Vec::new(),
            checked: 0,
        };
        check.visit_query(&query);
        checked += check.checked;
    }
    assert!(checked > 500, "only {checked} expressions checked");
}

#[test]
fn spans_cover_operands_literals_and_parentheses() {
    let source = "RETURN a.b.c + 1 * (2 - x), 1 < n.num < 3, -9223372036854775808, true, null";
    let query = decypher::parse(source).unwrap();
    let mut texts = Vec::new();
    struct Texts<'s, 'v>(&'s str, &'v mut Vec<&'s str>);
    impl<'a> Visit<'a> for Texts<'_, '_> {
        fn visit_expression(&mut self, e: &'a Expression) {
            let s = e.span();
            self.1.push(&self.0[s.start..s.end]);
            walk_expression(self, e);
        }
    }
    Texts(source, &mut texts).visit_query(&query);
    for expected in [
        "a.b.c + 1 * (2 - x)",
        "a.b.c",
        "a.b",
        "(2 - x)",
        "1 < n.num < 3",
        "n.num",
        "-9223372036854775808",
        "true",
        "null",
    ] {
        assert!(
            texts.contains(&expected),
            "no expression spans `{expected}`: {texts:?}"
        );
    }
}

/// An item without `AS` is named by its text, which ends at its last token:
/// whitespace and comments that follow are no part of the column's name, so
/// two items that differ only in them are duplicates.
#[cfg(feature = "hir")]
#[test]
fn unaliased_column_names_end_at_the_last_token() {
    for (source, expected) in [
        ("MATCH (a) RETURN a.age + 1 LIMIT 1", vec!["a.age + 1"]),
        (
            "RETURN 1 + 1 /* c */ , NOT true\n",
            vec!["1 + 1", "NOT true"],
        ),
        (
            "MATCH (a) RETURN a.x < 3   , a.x IN [1]  , -a.x // c\n",
            vec!["a.x < 3", "a.x IN [1]", "-a.x"],
        ),
        // Trivia inside an item stays part of its name.
        ("RETURN 1   +   1   ", vec!["1   +   1"]),
    ] {
        let hir = decypher::analyze(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        let columns = hir
            .parts
            .iter()
            .flat_map(|part| &part.operations)
            .find_map(|op| match op {
                decypher::hir::ops::Operation::Return(r) => Some(&r.columns),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{source:?}: no Return"));
        let names: Vec<&str> = columns
            .iter()
            .map(|&id| hir.arenas.bindings.get(id).name.as_str())
            .collect();
        assert_eq!(names, expected, "{source:?}");
    }
    for source in [
        "RETURN 1 + 1 , 1 + 1   ",
        "MATCH (a) RETURN a.x < 3, a.x < 3 /* again */",
    ] {
        assert!(
            decypher::analyze(source).is_err(),
            "{source:?} names a column twice"
        );
    }
}
