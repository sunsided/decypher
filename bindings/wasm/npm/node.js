// Node.js entry point.
//
// The browser build loads its WebAssembly module with `fetch()`, which Node
// cannot do for `file:` URLs. Instantiate it synchronously from disk instead so
// the API is usable right after `import`; calling the default `init()` export
// afterwards is a harmless no-op, which keeps code portable between runtimes.
import { readFileSync } from "node:fs";
import { initSync } from "./web/decypher.js";

initSync({ module: readFileSync(new URL("./web/decypher_bg.wasm", import.meta.url)) });

export * from "./web/decypher.js";
export { default } from "./web/decypher.js";
