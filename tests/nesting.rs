//! Deeply nested input is rejected with `NestingTooDeep` instead of
//! overflowing the stack, which would abort the process.

use decypher::ErrorKind;

/// Queries nesting each recursive construct `n` levels deep.
fn nested(n: usize) -> Vec<(&'static str, String)> {
    vec![
        (
            "parentheses",
            format!("RETURN {}1{}", "(".repeat(n), ")".repeat(n)),
        ),
        (
            "lists",
            format!("RETURN {}1{}", "[".repeat(n), "]".repeat(n)),
        ),
        (
            "maps",
            format!("RETURN {}1{}", "{a:".repeat(n), "}".repeat(n)),
        ),
        (
            "calls",
            format!("RETURN {}1{}", "abs(".repeat(n), ")".repeat(n)),
        ),
        ("NOT", format!("RETURN {}true", "NOT ".repeat(n))),
        (
            "CALL subqueries",
            format!(
                "{}RETURN 1 AS x{}",
                "CALL { ".repeat(n),
                " } RETURN x".repeat(n)
            ),
        ),
        (
            "EXISTS",
            format!(
                "MATCH (n) WHERE {}true{} RETURN n",
                "EXISTS { MATCH (m) WHERE ".repeat(n),
                " }".repeat(n)
            ),
        ),
        (
            "label expressions",
            format!("MATCH (n:{}A{}) RETURN n", "(".repeat(n), ")".repeat(n)),
        ),
        (
            "label negation",
            format!("MATCH (n:{}A) RETURN n", "!".repeat(n)),
        ),
        (
            "patterns",
            format!("MATCH {}(a)-->(b){} RETURN a", "(".repeat(n), ")".repeat(n)),
        ),
        (
            "FOREACH",
            format!(
                "MATCH (n) {}SET n.x = 1{}",
                "FOREACH (x IN [1] | ".repeat(n),
                ")".repeat(n)
            ),
        ),
        (
            "list comprehensions",
            format!("RETURN {}1{}", "[x IN ".repeat(n), " | x]".repeat(n)),
        ),
        // Operator chains nest in the AST without nesting in the parser.
        ("additions", format!("RETURN 1{}", " + 1".repeat(n))),
        (
            "property lookups",
            format!("WITH {{a: 1}} AS m RETURN m{}", ".a".repeat(n)),
        ),
        ("index chains", format!("RETURN [1]{}", "[0]".repeat(n))),
    ]
}

#[test]
fn deep_nesting_is_an_error_not_a_stack_overflow() {
    for (what, query) in nested(50_000) {
        let err = decypher::parse(query.as_str()).expect_err(what);
        assert!(
            matches!(err.kind, ErrorKind::NestingTooDeep { .. }),
            "{what}: {}",
            err.kind
        );
        #[cfg(feature = "hir")]
        assert!(decypher::analyze(query.as_str()).is_err(), "{what}");

        // The CST stays lossless: after a nesting error the unparsed rest is
        // kept in an error node. (Operator chains do not nest in the CST.)
        let cst = decypher::parse_cst(query.as_str());
        assert_eq!(cst.tree.text().to_string(), query, "{what}");
    }
}

/// The deepest version of each construct the parser accepts goes through
/// the whole pipeline on the test thread's default stack (2 MB, in a debug
/// build), so a change that grows a recursive stack frame fails here rather
/// than in a user's process.
#[test]
fn deepest_accepted_input_fits_the_default_stack() {
    let names: Vec<_> = nested(1).into_iter().map(|(what, _)| what).collect();
    for what in names {
        let query = |n: usize| nested(n).into_iter().find(|(w, _)| *w == what).unwrap().1;
        // Every probed depth is either within the limits or rejected, so
        // probing is safe on this thread too.
        let accepted = |n: usize| decypher::parse(query(n).as_str()).is_ok();
        let (mut lo, mut hi) = (1usize, 400usize);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if accepted(mid) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        assert!(lo >= 10, "{what}: only {lo} levels accepted");
        let deepest = query(lo);
        let ast = decypher::parse(deepest.as_str()).unwrap_or_else(|e| panic!("{what}: {e}"));
        let _ = decypher::ast::ToCypher::to_cypher(&ast);
        #[cfg(feature = "hir")]
        let _ = decypher::analyze(deepest.as_str());
    }
}
