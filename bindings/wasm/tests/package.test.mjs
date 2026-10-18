// Tests for the built npm package in `pkg/`. Run with `node --test tests/*.test.mjs`
// after `node scripts/build.mjs`.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { describe, it } from "node:test";

import init, { cst, format, parse, validate, version } from "../pkg/node.js";

describe("node entry point", () => {
  it("is usable without awaiting init", () => {
    assert.match(version(), /^\d+\.\d+\.\d+/);
  });

  it("treats init() as a no-op", async () => {
    await init();
    assert.equal(parse("RETURN 1").ok, true);
  });
});

describe("parse", () => {
  it("returns the typed AST", () => {
    const result = parse("MATCH (n:Person) RETURN n.name AS name");
    assert.equal(result.ok, true);
    assert.equal(result.ast.statements.length, 1);
    assert.deepEqual(result.ast.span, { start: 0, end: 38 });
    assert.ok("SingleQuery" in result.ast.statements[0]);
  });

  it("reports every syntax error with positions", () => {
    const result = parse("MATCH (n RETURN n;\nRETURN");
    assert.equal(result.ok, false);
    assert.ok(result.errors.length >= 2);

    const [first] = result.errors;
    assert.match(first.message, /expected `\)`/);
    assert.deepEqual(first.span, { start: 9, end: 15 });
    assert.equal(first.line, 1);
    assert.equal(first.column, 10);
    assert.match(first.rendered, /\^\^\^\^\^\^/);
  });

  it("reports UTF-16 ranges that slice JavaScript strings", () => {
    // The emoji takes 4 bytes in UTF-8 but only 2 UTF-16 code units.
    const source = "RETURN '😀' AS )";
    const [error] = parse(source).errors;
    assert.deepEqual(error.span, { start: 17, end: 18 });
    assert.deepEqual(error.range, { start: 15, end: 16 });
    assert.equal(source.slice(error.range.start, error.range.end), ")");
  });

  it("rejects empty input", () => {
    assert.equal(parse("   ").ok, false);
  });
});

describe("validate", () => {
  it("accepts a well-formed query", () => {
    assert.deepEqual(validate("MATCH (n) RETURN n"), { ok: true });
  });

  it("finds unresolved variables the parser accepts", () => {
    assert.equal(parse("MATCH (n) RETURN m").ok, true);

    const result = validate("MATCH (n) RETURN m");
    assert.equal(result.ok, false);
    assert.match(result.errors[0].message, /unresolved variable `m`/);
  });

  it("enforces WITH scoping", () => {
    const result = validate("MATCH (p)-->(m) WITH p, count(m) AS c RETURN m");
    assert.equal(result.ok, false);
    assert.match(result.errors[0].message, /unresolved variable `m`/);
  });

  it("finds implicit grouping keys", () => {
    const result = validate("MATCH (n) RETURN n.x + count(n)");
    assert.equal(result.ok, false);
    assert.match(result.errors[0].message, /non-grouping keys: \["n\.x"\]/);
    assert.deepEqual(
      validate("MATCH (n) WITH toUpper(n.name) AS k, count(*) AS c RETURN k, c"),
      { ok: true },
    );
  });
});

describe("format", () => {
  it("normalises keywords and whitespace", () => {
    assert.deepEqual(format("match (n)\n   return  n limit 3"), {
      ok: true,
      cypher: "MATCH (n) RETURN n LIMIT 3;",
    });
  });
});

describe("cst", () => {
  it("is lossless, including comments", () => {
    const source = "RETURN 1 // answer";
    const { tree, errors } = cst(source);
    assert.deepEqual(errors, []);
    assert.equal(tree.kind, "SOURCE_FILE");

    const tokens = [];
    const walk = (element) =>
      "children" in element ? element.children.forEach(walk) : tokens.push(element.text);
    walk(tree);
    assert.equal(tokens.join(""), source);
  });

  it("still produces a tree for invalid input", () => {
    const { tree, errors } = cst("MATCH (n RETURN n");
    assert.equal(tree.kind, "SOURCE_FILE");
    assert.ok(errors.length > 0);
  });
});

describe("browser entry point", () => {
  it("initialises from explicit bytes", () => {
    // Run in a fresh process: in this one the Node entry already instantiated
    // the shared module, which would turn init() into a no-op.
    const script = `
      import { readFile } from "node:fs/promises";
      import init, { parse } from "./pkg/web/decypher.js";
      await init({ module_or_path: await readFile("./pkg/web/decypher_bg.wasm") });
      if (!parse("RETURN 1").ok) process.exit(1);
    `;
    const cwd = new URL("..", import.meta.url);
    const child = spawnSync(process.execPath, ["--input-type=module", "-e", script], { cwd });
    assert.equal(child.status, 0, child.stderr.toString());
  });
});
