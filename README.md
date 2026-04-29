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
- **`serde` support** — Optional `serde` feature for `Serialize`/`Deserialize` derives on all AST nodes.
- **Low-level escape hatch** — The `low-level` feature re-exports the raw pest `Rule` and `Pairs` types for advanced use.

## Stability

The AST is **unstable** until 0.2.0. Grammar completeness is tracked against the openCypher EBNF. Unsupported grammar productions return `CypherError::Unsupported` rather than panicking.

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
