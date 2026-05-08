# cypher-rs

Parse Neo4j® 5 Cypher® and [openCypher](https://opencypher.org/) queries using Rust.

This project is independent and is not affiliated with, endorsed by, or sponsored by Neo4j, Inc.
Cypher® and Neo4j® are registered trademarks of Neo4j, Inc.

<div align="center">
  <img src="https://raw.githubusercontent.com/sunsided/cypher/refs/heads/main/.readme/banner.png" alt="Cypher crate hero picture" />
</div>

`cypher-rs` provides a typed AST for Cypher® and openCypher queries, built on a hand-written error-resilient rowan parser derived from the openCypher EBNF specification.

> **Note**: This project is a fork and complete re-implementation of the [original pest-based parser](https://github.com/a-poor/open-cypher) by [Austin Poor](https://github.com/a-poor). The parser has been rewritten from the ground up using [rowan](https://github.com/rust-analyzer/rowan) instead of pest.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
cypher = "0.2"
```

```rust
use cypher_rs::parse;

let query = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name;").unwrap();
println!("{:#?}", query);
```

## Features

- **Typed AST** — Every Cypher construct maps to a Rust enum or struct with full type safety.
- **Source spans** — Every AST node carries a `Span { start, end }` (byte offsets into the input) for diagnostics.
- **Ergonomic errors** — `CypherError` with syntax, AST build, and unsupported-production variants via `thiserror`.
- **Cypher emission** — The `ToCypher` trait renders any AST node back into valid openCypher text, enabling round-trips (`parse → ast → to_cypher → parse`).
- **`serde` support** — Optional `serde` feature for `Serialize`/`Deserialize` derives on all AST nodes.
- **Typed CST (unstable)** — A rust-analyzer-style typed wrapper layer over a lossless rowan CST, available under `cypher_rs::cst`. Each CST node (`SourceFile`, `MatchClause`, `Expression`, …) exposes typed accessor methods instead of raw `SyntaxKind` matches. This is what the public `parse()` function uses internally.

### Typed CST example

```rust
use cypher_rs::cst::{parse, AstNode, BinOp, Expression};

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
use cypher_rs::ast::ToCypher;
use cypher_rs::parse;

let query = parse("MATCH (n:Person) WHERE n.age > 18 RETURN n.name;").unwrap();
let cypher = query.to_cypher();
check!(cypher.contains("MATCH"));
```

## Stability

The AST is **unstable** until 0.2.0. Grammar completeness is tracked against the openCypher EBNF. Unsupported grammar productions return `CypherError::Unsupported` rather than panicking.

The `ToCypher` trait and round-trip guarantees are also **unstable** — formatting output may change between releases until 0.2.0.

## Project Structure

- `src/parser/` — Hand-written error-resilient rowan parser (grammar + lexer).
- `src/syntax/` — Rowan language definition and typed CST wrappers.
- `src/ast/` — Typed AST node definitions and the CST → AST lowering logic (`src/ast/build_cst/`).
- `src/error.rs` — `CypherError` and `Span` types.
- `src/lib.rs` — Public API (`parse`, `Query`, `CypherError`).
- `assets/cypher.ebnf` — OpenCypher grammar definition from the openCypher site.

## Contributing

Contributions of any size are welcome! Please feel free to submit issues or PRs.

## License

This library code is **EUPL-1.2**, **MIT**, or **Apache-2.0**.

Cypher® and Neo4j® are registered trademarks of Neo4j, Inc.
