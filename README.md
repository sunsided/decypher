# decypher

A Rust parser for the Cypher® property graph language, based on the [openCypher](https://opencypher.org/) specification.

This project is independent and is not affiliated with, endorsed by, or sponsored by Neo4j, Inc.
Cypher® and Neo4j® are registered trademarks of Neo4j, Inc.

<div align="center">
  <img src="https://raw.githubusercontent.com/sunsided/cypher/refs/heads/main/.readme/banner.png" alt="Cypher crate hero picture" />
</div>

`decypher` provides a typed AST for Cypher® and openCypher queries, built on a hand-written error-resilient rowan parser derived from the openCypher EBNF specification, and a scope-resolved intermediate representation (HIR) with semantic checks for tools that analyze or execute queries.

> **Note**: This project is a fork and complete re-implementation of the [original pest-based parser](https://github.com/a-poor/open-cypher) by [Austin Poor](https://github.com/a-poor). The parser has been rewritten from the ground up using [rowan](https://github.com/rust-analyzer/rowan) instead of pest.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
decypher = "0.4"
```

```rust
use decypher::parse;

let query = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name;").unwrap();
println!("{:#?}", query);
```

## Features

- **Typed AST** — Every Cypher construct maps to a Rust enum or struct with full type safety.
- **Modern Cypher and GQL syntax** — Besides openCypher 9: quantified path patterns (`((a)-->(b)){1,3}`, `-[:R]->+`), path selectors (`shortestPath`, `SHORTEST k`, `ALL SHORTEST`), inline `WHERE` in patterns, label expressions (`:A|B`, `:!A`, `:%`, `IS Person`), dynamic labels and types (`:$(expr)`, also in `SET` and `REMOVE`), `COUNT`/`COLLECT`/`EXISTS` subqueries, type and normalization predicates, `||` concatenation and `trim(BOTH 'x' FROM s)`.
- **HIR and semantic analysis** — `analyze` lowers a query into a scope-resolved HIR (`decypher::hir`): every variable resolved to a binding, patterns normalized into graphs, projections split into grouping keys and aggregates, and the result columns in output order. Along the way it reports undefined and redeclared variables, invalid aggregations, type errors in literals and known operands, and wrong calls of known functions and procedures.
- **Source spans** — Every AST node carries a `Span { start, end }` (byte offsets into the input) for diagnostics.
- **Bounded nesting** — Untrusted input cannot overflow the stack. Brackets, function calls, `NOT`, patterns and list comprehensions nest at most 127 levels, `CALL { … }` subqueries 15, `EXISTS { … }` 14 and label expressions 30; operator chains such as `a + b + …` hold at most 255 operators and label alternatives such as `:A|B|…` 256. Anything deeper is rejected with `ErrorKind::NestingTooDeep`. Within these limits parsing and analysis fit a 2 MB thread stack even in debug builds.
- **Ergonomic errors** — `CypherError` with syntax, AST build, and unsupported-production variants; it implements `std::error::Error`. `parse_all` collects every syntax error instead of stopping at the first, and the optional `miette` feature renders them as rich diagnostics.
- **Cypher emission** — The `ToCypher` trait renders any AST node back into valid openCypher text, enabling round-trips (`parse → ast → to_cypher → parse`).
- **`serde` support** — Optional `serde` feature for `Serialize` derives on all AST nodes and spans.
- **Typed CST (unstable)** — A rust-analyzer-style typed wrapper layer over a lossless rowan CST, available under `decypher::cst`. Each CST node (`SourceFile`, `MatchClause`, `Expression`, …) exposes typed accessor methods instead of raw `SyntaxKind` matches. This is what the public `parse()` function uses internally.

### Typed CST example

```rust
use decypher::cst::parse;

let result = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name");
let source = result.tree();

for stmt in source.statements() {
    for clause in stmt.clauses() {
        // use pattern matching on the Clause enum
    }
}
```

## Emitting Cypher

The `ToCypher` trait converts AST nodes back into openCypher text. This is useful for query rewriting, formatting, and round-trip testing.

```rust
use decypher::ast::ToCypher;
use decypher::parse;

let query = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name;").unwrap();
let cypher = query.to_cypher();
assert!(cypher.contains("MATCH"));
```

## Analyzing queries

`analyze` parses and lowers a query in one step (feature `hir`, on by default). The HIR keeps the query's meaning, not its spelling: variables are bindings in arenas, and each query part is a list of operations.

```rust
use decypher::hir::Operation;

let hir = decypher::analyze(
    "MATCH (a)-[:KNOWS]->(b) RETURN count(b) AS friends, a.name AS name",
)
.unwrap();

// The result columns, in the order the RETURN wrote them.
let part = hir.parts.last().unwrap();
if let Some(Operation::Return(ret)) = part.operations.last() {
    let columns: Vec<&str> = ret
        .columns
        .iter()
        .map(|&id| hir.arenas.bindings.get(id).name.as_str())
        .collect();
    assert_eq!(columns, ["friends", "name"]);
}
```

Function and procedure calls are checked against a `sema::Catalog`, seeded with the built-in functions of Neo4j 5, Cypher 25 and GQL. Register plugin functions, aggregates and procedures on it; a strict catalog also reports unknown ones:

```rust
use decypher::hir::LowerConfig;
use decypher::sema::Catalog;

let mut catalog = Catalog::default();
catalog.set_strict(true).register_aggregate("my.median");
let config = LowerConfig { catalog };

assert!(decypher::analyze_with_config("RETURN nope(1)", &config).is_err());
assert!(decypher::analyze_with_config("MATCH (n) RETURN my.median(n.age)", &config).is_ok());
```

`decypher::sema::analyze` runs the same checks on the AST alone, without building the HIR.

## Stability

From 0.2.0 on, the public API follows semantic versioning: there are no breaking changes within a minor line such as 0.4.x, and the next breaking release is 0.5.0. That covers the typed AST (`decypher::ast`), `ToCypher`, the HIR (`decypher::hir`), semantic analysis (`decypher::sema`) and the error types, with these provisos:

- **`ErrorKind` may gain variants** (it is `#[non_exhaustive]`), and error messages may be worded differently.
- **`ToCypher` formatting may change** (whitespace, keyword case), but printing a parsed query and parsing the result always gives back the same AST; the `fuzz_parse_roundtrip` fuzz target checks this.
- **The nesting limits** (see *Bounded nesting* above) may be raised, but not lowered.
- **The typed CST (`decypher::cst`) stays unstable** and may change in any release.

Grammar completeness is tracked against the openCypher EBNF. Unsupported grammar productions return `CypherError::Unsupported` rather than panicking.

## JavaScript / WebAssembly

The parser is also available as an npm package for Node.js and browsers,
built from [`bindings/wasm`](bindings/wasm/README.md). Try it in the
[online playground](https://sunsided.github.io/decypher/).

```js
import init, { parse, validate } from "@sunsided/decypher";

await init(); // needed in browsers; a no-op in Node.js
const result = validate("MATCH (n) RETURN m");
if (!result.ok) console.error(result.errors[0].rendered);
```

## Project Structure

- `src/parser/` — Hand-written error-resilient rowan parser (grammar + lexer).
- `src/syntax/` — Rowan language definition and typed CST wrappers.
- `src/ast/` — Typed AST node definitions and the CST → AST lowering logic (`src/ast/build_cst/`).
- `src/hir/` — The HIR and the AST → HIR lowering (`analyze`).
- `src/sema/` — Semantic checks on the AST: name resolution, aggregation, types and the function catalog.
- `src/error.rs` — `CypherError` and `Span` types.
- `src/lib.rs` — Public API (`parse`, `parse_all`, `analyze`, `Query`, `CypherError`).
- `bindings/wasm/` — WebAssembly bindings and the npm package build.
- `assets/cypher.ebnf` — OpenCypher grammar definition from the openCypher site.

## Contributing

Contributions of any size are welcome! Please feel free to submit issues or PRs.

## License

This library code is triple-licensed under **EUPL-1.2**, **MIT**, or **Apache-2.0**; choose whichever suits your use case best.

Cypher® and Neo4j® are registered trademarks of Neo4j, Inc.
