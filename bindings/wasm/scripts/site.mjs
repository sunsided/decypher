#!/usr/bin/env node
// Assembles the browser playground as a static site in `bindings/wasm/site/`,
// ready for GitHub Pages or any static host. Run `scripts/build.mjs` first.
import { cpSync, existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const crateDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const webDir = join(crateDir, "pkg", "web");
const siteDir = join(crateDir, "site");

if (!existsSync(join(webDir, "decypher_bg.wasm"))) {
  throw new Error("pkg/ is missing; run `node scripts/build.mjs` first");
}

rmSync(siteDir, { recursive: true, force: true });
cpSync(join(crateDir, "examples", "browser"), siteDir, { recursive: true });
for (const file of ["decypher.js", "decypher_bg.wasm"]) {
  cpSync(join(webDir, file), join(siteDir, "pkg", file));
}

// In the repository the page reaches the package two levels up; on the site
// it sits next to it.
const indexPath = join(siteDir, "index.html");
const index = readFileSync(indexPath, "utf8");
const rewritten = index.replace('"../../pkg/web/decypher.js"', '"./pkg/decypher.js"');
if (rewritten === index) throw new Error("import map entry not found in index.html");
writeFileSync(indexPath, rewritten);

// Serve files as-is instead of running them through Jekyll.
writeFileSync(join(siteDir, ".nojekyll"), "");

console.log(`site assembled in ${siteDir}`);
