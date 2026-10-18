#!/usr/bin/env node
// cypher-lint: validate Cypher files and summarise what each query touches.
//
//   node lint.js queries/*.cypher
//   echo 'MATCH (n) RETURN m' | node lint.js -
//   node lint.js --format query.cypher
//
// Exits with status 1 if any file has errors.
import { readFileSync } from "node:fs";
import { parseArgs } from "node:util";

// The Node entry point instantiates the WebAssembly module on import, so the
// functions are ready to call straight away.
import { format, parse, validate } from "@sunsided/decypher";

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    format: { type: "boolean", default: false },
  },
});

if (positionals.length === 0) {
  console.error("usage: lint.js [--format] FILE... (use - for stdin)");
  process.exit(2);
}

let failed = false;
for (const file of positionals) {
  const source = readFileSync(file === "-" ? 0 : file, "utf8");
  failed = !lint(file === "-" ? "<stdin>" : file, source) || failed;
}
process.exit(failed ? 1 : 0);

/**
 * @param {string} name
 * @param {string} source
 * @returns {boolean} whether the file is free of errors
 */
function lint(name, source) {
  // `validate` runs the parser *and* name resolution, so it also catches
  // queries such as `RETURN m` where `m` was never bound or fell out of scope.
  const checked = validate(source);
  if (!checked.ok) {
    for (const error of checked.errors) {
      // `rendered` is a ready-made report with a source excerpt; the
      // structured fields (line, column, span, range, notes) are there for
      // editors and CI annotations.
      console.error(error.rendered.replace("query:", `${name}:`));
    }
    return false;
  }

  // Validation passed, so parsing cannot fail here.
  const { ast } = /** @type {{ ast: import("@sunsided/decypher").Query }} */ (parse(source));
  const summary = summarise(ast);

  console.log(`✔ ${name}: ${ast.statements.length} statement(s)`);
  for (const [title, items] of Object.entries(summary)) {
    if (items.size > 0) console.log(`    ${title.padEnd(11)} ${[...items].sort().join(", ")}`);
  }
  if (values.format) {
    const formatted = format(source);
    if (formatted.ok) console.log(`    formatted   ${formatted.cypher}`);
  }
  return true;
}

/**
 * Collect the labels, relationship types, properties and parameters a query
 * refers to by walking the serialised AST.
 *
 * Rust enums arrive externally tagged (`{ "Static": { ... } }`), structs as
 * plain objects, and vectors as arrays, so a small generic walker that tracks
 * the key it came through is enough to pick out the interesting nodes.
 *
 * @param {unknown} ast
 */
function summarise(ast) {
  const summary = {
    labels: new Set(),
    relTypes: new Set(),
    properties: new Set(),
    parameters: new Set(),
  };

  /** @param {any} value @param {string[]} path */
  const walk = (value, path) => {
    if (Array.isArray(value)) {
      value.forEach((item) => walk(item, path));
      return;
    }
    if (value === null || typeof value !== "object") return;

    const parent = path.at(-1);
    if (parent === "Static" && path.includes("labels")) summary.labels.add(value.name);
    if (parent === "Static" && path.includes("types")) summary.relTypes.add(value.name);
    if (parent === "property") summary.properties.add(value.name.name);
    if (parent === "Parameter") summary.parameters.add(`$${value.name.name}`);

    for (const [key, child] of Object.entries(value)) walk(child, [...path, key]);
  };

  walk(ast, []);
  return summary;
}
