use assert2::check;
use cypher::parse;

fn run_suite(name: &str, source: &str) {
    let queries: Vec<String> = source
        .split("\n// ")
        .skip(1)
        .filter_map(|block| {
            let first_newline = block.find('\n')?;
            let query = block[first_newline..].trim();
            if query.is_empty() {
                None
            } else {
                Some(query.to_string())
            }
        })
        .collect();

    for (i, query) in queries.iter().enumerate() {
        let result = parse(query);
        check!(
            result.is_ok(),
            "rowan parser — Suite {} query {} failed:\n{}\nError: {:?}",
            name,
            i + 1,
            query,
            result.err()
        );
    }
}

fn run_suite_expect_fail(name: &str, source: &str) {
    let queries: Vec<String> = source
        .split("\n// ")
        .skip(1)
        .filter_map(|block| {
            let first_newline = block.find('\n')?;
            let query = block[first_newline..].trim();
            if query.is_empty() {
                None
            } else {
                Some(query.to_string())
            }
        })
        .collect();

    for (i, query) in queries.iter().enumerate() {
        let result = parse(query);
        check!(
            result.is_err(),
            "rowan parser — Suite {} query {} unexpectedly succeeded (expected failure):\n{}",
            name,
            i + 1,
            query
        );
    }
}

#[test]
fn suite_1() {
    run_suite("1", include_str!("suite-1.cypher"));
}

#[test]
fn suite_2() {
    run_suite("2", include_str!("suite-2.cypher"));
}

#[test]
fn suite_3() {
    run_suite("3", include_str!("suite-3.cypher"));
}

#[test]
fn suite_4() {
    run_suite("4", include_str!("suite-4.cypher"));
}

#[test]
fn suite_5() {
    run_suite_expect_fail("5", include_str!("suite-5.cypher"));
}
