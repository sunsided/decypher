# @sunsided/decypher

The [decypher](https://github.com/sunsided/decypher) Cypher parser compiled to
WebAssembly, for Node.js (18+) and browsers. It understands openCypher plus the
Neo4j 5 and GQL extensions decypher supports, and gives you:

| Function                    | Returns                                                            |
| --------------------------- | ------------------------------------------------------------------ |
| `parse(source)`             | the typed AST as plain JSON                                        |
| `validate(source)`          | syntax errors, unresolved / out-of-scope variables and invalid aggregations, no tree |
| `format(source)`            | the query re-emitted as canonical single-line Cypher               |
| `cst(source)`               | the lossless syntax tree (every token, whitespace and comments)    |
| `version()`                 | the package version                                                |

Invalid Cypher never throws. `parse`, `validate` and `format` return
`{ ok: true, ... }` or `{ ok: false, errors }`; `cst` always returns a tree
plus whatever `errors` it found.

## Installation

The package is published to this repository's
[GitHub Packages](https://github.com/sunsided/decypher/pkgs/npm/decypher)
npm registry. Point the `@sunsided` scope at it in your project's `.npmrc`:

```ini
@sunsided:registry=https://npm.pkg.github.com
```

GitHub Packages requires authentication even for public packages: log in
with `npm login --scope=@sunsided --auth-type=legacy --registry=https://npm.pkg.github.com`
using a token with the `read:packages` scope as password, or add
`//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}` to `.npmrc`. Then:

```sh
npm install @sunsided/decypher@alpha
```

Pre-releases are published under their dist-tag (`alpha`), so name it until
a stable version exists.

## Usage

```js
import init, { parse, validate, format } from "@sunsided/decypher";

// Browsers must instantiate the module once before calling anything. In
// Node.js the module is ready on import and this call does nothing, so the
// same code runs in both.
await init();

const result = parse("MATCH (p:Person)-[:KNOWS]->(f) RETURN f.name");
if (result.ok) {
  console.log(result.ast.statements.length); // 1
} else {
  for (const error of result.errors) console.error(error.rendered);
}

validate("MATCH (p)-->(m) WITH p RETURN m");
// { ok: false, errors: [{ message: "unresolved variable `m`", line: 1, column: 31, ... }] }

format("match (n)  return n limit 3");
// { ok: true, cypher: "MATCH (n) RETURN n LIMIT 3;" }
```

TypeScript declarations ship with the package (`Diagnostic`, `Result<T>`,
`CstNode`, …).

### Diagnostics

Each error carries:

- `message` — one line, e.g. ``unexpected token `RETURN`, expected `)` ``
- `line`, `column` — 1-based position of the start
- `range` — UTF-16 offsets; `source.slice(range.start, range.end)` is the
  offending text, and they map straight onto editor positions
- `span` — the same range as UTF-8 byte offsets
- `notes` — extra `info` / `warning` / `help` messages
- `rendered` — a multi-line report with a source excerpt and `^^^` marker

### The AST

`parse` returns decypher's [`ast::Query`](https://docs.rs/decypher/latest/decypher/ast/)
serialised with serde, so the Rust docs describe every node. Two conventions to
know:

- **Enums are externally tagged.** A variant becomes an object with a single
  key: `{ "Match": { ... } }`, `{ "Static": { "name": "Person", ... } }`.
  Unit variants are plain strings (`"Right"`, `"Gt"`).
- **Spans are UTF-8 byte offsets**, as in Rust. For ASCII queries they equal
  JavaScript string indices; otherwise convert with
  `new TextDecoder().decode(new TextEncoder().encode(source).slice(start, end))`,
  or use the `range` of diagnostics, which is already UTF-16.

A small generic walker usually beats matching the full shape; see
[`examples/node/lint.js`](examples/node/lint.js), which collects every label,
relationship type, property and parameter a query touches in ~20 lines.

### Bundlers and custom loading

The browser build loads `decypher_bg.wasm` relative to its own module via
`new URL(..., import.meta.url)`, which Vite, webpack 5, esbuild, Rollup and
Parcel all understand. To host the binary elsewhere, pass its location:

```js
import init from "@sunsided/decypher";
import wasmUrl from "@sunsided/decypher/decypher_bg.wasm?url"; // Vite syntax

await init({ module_or_path: wasmUrl });
```

`init` also accepts a `Response`, an `ArrayBuffer`/`Uint8Array` or a compiled
`WebAssembly.Module`; `initSync({ module })` does the same synchronously.

## Building the package

Prerequisites: Rust (see `rust-version` in `Cargo.toml`), the
`wasm32-unknown-unknown` target, [`wasm-pack`](https://github.com/drager/wasm-pack),
and Node.js 18+.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

task wasm:pack        # from the repository root; or, step by step:

cd bindings/wasm
node scripts/build.mjs          # release build into pkg/  (--dev for a debug build)
node --test tests/*.test.mjs    # test the built package
npm pack ./pkg --pack-destination dist   # dist/sunsided-decypher-<version>.tgz
```

`pkg/` is a complete npm package: one `web/` build of the bindings, plus a
`node.js` entry that loads the same `.wasm` from disk. `package.json`'s
`exports` points Node.js at `node.js` and everything else (browsers, bundlers,
Deno) at the web build.

## Consuming it without a registry

The tarball is self-contained, so you can hand it to anyone:

```sh
npm install ./sunsided-decypher-0.2.0-alpha.9.tgz
# or reference it from package.json:
#   "dependencies": { "@sunsided/decypher": "file:../path/to/sunsided-decypher-0.2.0-alpha.9.tgz" }
```

While working on the bindings themselves, depend on the build directory
instead (`"file:../decypher/bindings/wasm/pkg"`) and rebuild to pick up changes.
CI uploads the tarball as the `npm-package` artifact of every run.

## Examples

### Node.js: `cypher-lint`

[`examples/node`](examples/node) validates `.cypher` files and summarises what
each query touches:

```sh
task wasm:build
cd bindings/wasm/examples/node
npm install
node lint.js --format queries/*.cypher
```

```text
error: unresolved variable `m`
  --> queries/broken.cypher: 4:16
   |
 4 | RETURN p.name, m.title;
   |                ^

✔ queries/friends.cypher: 1 statement(s)
    labels      Person
    relTypes    KNOWS
    properties  age, name
    parameters  $minAge, $name
    formatted   MATCH (a:Person {name: $name})-[:KNOWS]->(b:Person) WHERE ...
```

### Browser: playground

[`examples/browser`](examples/browser) is a dependency-free page, deployed to
<https://sunsided.github.io/decypher/> on every push to `main`, with live
syntax highlighting driven by the CST, inline error squiggles, and AST / CST /
formatted views. It uses an import map, so no bundler is needed:

```sh
task wasm:build
python3 -m http.server -d bindings/wasm 8000
# open http://localhost:8000/examples/browser/
```

`task wasm:serve` instead serves the exact site that is deployed
(`scripts/site.mjs` assembles it into `site/`).
