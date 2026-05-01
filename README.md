# open-cypher

Parse [openCypher](https://opencypher.org/) queries using Rust.

`open-cypher` provides a typed AST for openCypher queries, built on top of a [pest](https://pest.rs/) grammar derived from the openCypher EBNF specification.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
open-cypher = "0.2"
```

```rust
use open_cypher::parse;

let query = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name;").unwrap();
println!("{:#?}", query);
```

## Features

- **Typed AST** — Every Cypher construct maps to a Rust enum or struct with full type safety.
- **Source spans** — Every AST node carries a `Span { start, end }` (byte offsets into the input) for diagnostics.
- **Ergonomic errors** — `CypherError` with syntax, AST build, and unsupported-production variants via `thiserror`.
- **Cypher emission** — The `ToCypher` trait renders any AST node back into valid openCypher text, enabling round-trips (`parse → ast → to_cypher → parse`).
- **`serde` support** — Optional `serde` feature for `Serialize`/`Deserialize` derives on all AST nodes.
- **Low-level escape hatch** — The `low-level` feature re-exports the raw pest `Rule` and `Pairs` types for advanced use.
- **Typed CST (unstable)** — A rust-analyzer-style typed wrapper layer over a lossless rowan CST, available under `open_cypher::cst`. Each CST node (`SourceFile`, `MatchClause`, `Expression`, …) exposes typed accessor methods instead of raw `SyntaxKind` matches. Pest remains the oracle for the public `parse()` API; the CST is provided for tooling and incremental migration.

### Typed CST example

```rust
use open_cypher::cst::{parse, AstNode, BinOp, Expression};

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
use open_cypher::ast::ToCypher;
use open_cypher::parse;

let query = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name;").unwrap();
let cypher = query.to_cypher();
assert!(cypher.contains("MATCH"));
```

## Stability

The AST is **unstable** until 0.2.0. Grammar completeness is tracked against the openCypher EBNF. Unsupported grammar productions return `CypherError::Unsupported` rather than panicking.

The `ToCypher` trait and round-trip guarantees are also **unstable** — formatting output may change between releases until 0.2.0.

## Project Structure

- `src/cypher.pest` — The pest grammar file, based on the openCypher EBNF.
- `src/ast/` — Typed AST node definitions.
- `src/ast/build.rs` — Builder that converts pest parse trees into the typed AST.
- `src/error.rs` — `CypherError` and `Span` types.
- `src/lib.rs` — Public API (`parse`, `Query`, `CypherError`).
- `assets/cypher.ebnf` — OpenCypher grammar definition from the openCypher site.

## Contributing

Contributions of any size are welcome! Please feel free to submit issues or PRs.

## License

MIT OR Apache-2.0
