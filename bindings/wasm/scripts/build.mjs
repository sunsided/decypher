#!/usr/bin/env node
// Builds the npm package into `bindings/wasm/pkg/`.
//
//   node scripts/build.mjs           # release build
//   node scripts/build.mjs --dev     # unoptimised, with debug symbols
//
// Requires `wasm-pack` and the `wasm32-unknown-unknown` Rust target.
import { execFileSync } from "node:child_process";
import { cpSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const crateDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkgDir = join(crateDir, "pkg");
const webDir = join(pkgDir, "web");
const profile = process.argv.includes("--dev") ? "--dev" : "--release";

rmSync(pkgDir, { recursive: true, force: true });

// A single `web` build serves both runtimes: browsers and bundlers load it as
// is, and `node.js` instantiates the same .wasm from disk.
execFileSync(
  "wasm-pack",
  ["build", profile, "--target", "web", "--out-dir", webDir, "--out-name", "decypher", "--no-pack"],
  { cwd: crateDir, stdio: "inherit" },
);

// wasm-pack drops a `.gitignore` containing `*` into its output, which would
// make `npm pack` skip the whole directory.
rmSync(join(webDir, ".gitignore"), { force: true });

cpSync(join(crateDir, "npm", "node.js"), join(pkgDir, "node.js"));
cpSync(join(crateDir, "README.md"), join(pkgDir, "README.md"));
for (const license of ["LICENSE-APACHE", "LICENSE-EUPL", "LICENSE-MIT"]) {
  cpSync(join(crateDir, "..", "..", license), join(pkgDir, license));
}

// Ask cargo rather than parsing Cargo.toml, so workspace inheritance or
// reordered tables can't yield the wrong version.
const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    cwd: crateDir,
    encoding: "utf8",
  }),
);
const version = metadata.packages.find((p) => p.name === "decypher-wasm")?.version;
if (!version) throw new Error("decypher-wasm not found in `cargo metadata`");

const manifest = JSON.parse(readFileSync(join(crateDir, "npm", "package.json"), "utf8"));
manifest.version = version;
writeFileSync(join(pkgDir, "package.json"), JSON.stringify(manifest, null, 2) + "\n");

console.log(`built ${manifest.name}@${version} in ${pkgDir}`);
