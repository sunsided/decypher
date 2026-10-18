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

const cargoToml = readFileSync(join(crateDir, "Cargo.toml"), "utf8");
const version = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) throw new Error("could not read the package version from Cargo.toml");

const manifest = JSON.parse(readFileSync(join(crateDir, "npm", "package.json"), "utf8"));
manifest.version = version;
writeFileSync(join(pkgDir, "package.json"), JSON.stringify(manifest, null, 2) + "\n");

console.log(`built ${manifest.name}@${version} in ${pkgDir}`);
