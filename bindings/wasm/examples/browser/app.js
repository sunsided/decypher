import init, { analyze, cst, format, parse, validate, version } from "@sunsided/decypher";
import { EXAMPLES } from "./examples.js";

// In the browser the module must be instantiated before first use. `init()`
// fetches `decypher_bg.wasm` from next to `decypher.js`; pass a URL, Response
// or bytes to load it from somewhere else.
await init();

const source = document.querySelector("#source");
const highlight = document.querySelector("#highlight");
const diagnostics = document.querySelector("#diagnostics");
const output = document.querySelector("#output");
const tabs = document.querySelectorAll("[role=tab]");

document.querySelector("#version").textContent = `v${version()}`;

const picker = document.querySelector("#example");
picker.append(
  new Option("(edited)", "", false, false),
  ...EXAMPLES.map((example, i) => new Option(example.name, String(i))),
);
picker.options[0].hidden = true;
picker.addEventListener("change", () => {
  source.value = EXAMPLES[picker.value].query;
  source.setSelectionRange(0, 0);
  source.scrollTop = 0;
  update();
});
source.value = EXAMPLES[0].query;
picker.value = "0";

/** Everything derived from the current source; rebuilt by `update()`. */
let state;
/** The selected tab; the page picks the initial one in its markup. */
let view = document.querySelector("[role=tab][aria-selected=true]").dataset.view;
let showRawAst = false;
/** Highlights from hovering the output pane; win over `caretMarks`. */
let hoverMarks = [];
/** Occurrences of the variable under the caret. */
let caretMarks = [];
/** Open/closed state of AST tree nodes by path, kept across edits. */
const astOpen = new Map();
/** Parameter values typed into the Parameters tab, kept across edits. */
const paramValues = new Map();
/** AST tree nodes with a source span, for mapping the caret to the tree. */
let astNodes = [];

for (const tab of tabs) {
  tab.addEventListener("click", () => {
    view = tab.dataset.view;
    tabs.forEach((t) => t.setAttribute("aria-selected", String(t === tab)));
    hoverMarks = [];
    renderView();
    paintEditor();
  });
}

source.addEventListener("input", update);
source.addEventListener("scroll", () => {
  highlight.scrollTop = source.scrollTop;
  highlight.scrollLeft = source.scrollLeft;
});
document.addEventListener("selectionchange", () => {
  if (document.activeElement === source) onCaretMove();
});

function update() {
  const text = source.value;
  if (picker.value && text !== EXAMPLES[picker.value].query) picker.value = "";

  // The CST is lossless: concatenating its token texts reproduces the input
  // exactly, so it can drive highlighting even while the query is invalid.
  const tree = cst(text).tree;
  const checked = validate(text);
  const analyzed = analyze(text);
  const parsed = parse(text);

  state = {
    text,
    tree,
    tokens: flattenTokens(tree),
    errors: checked.ok ? [] : checked.errors,
    analysis: analyzed.ok ? analyzed : null,
    ast: parsed.ok ? parsed.ast : null,
    utf16: byteToUtf16(text),
  };

  hoverMarks = [];
  caretMarks = variableMarksAt(source.selectionStart);
  diagnostics.replaceChildren(...state.errors.map(renderDiagnostic));
  renderView();
  paintEditor();
}

// ---------------------------------------------------------------------------
// Editor overlay
// ---------------------------------------------------------------------------

/**
 * Flatten the CST into tokens with UTF-16 offsets (JavaScript string
 * indices) and their ancestor nodes, innermost last.
 */
function flattenTokens(tree) {
  const tokens = [];
  let offset = 0;
  const walk = (node, ancestors) => {
    for (const child of node.children) {
      if ("children" in child) {
        walk(child, [...ancestors, child]);
      } else {
        const start = offset;
        offset += child.text.length;
        tokens.push({ kind: child.kind, text: child.text, start, end: offset, ancestors });
      }
    }
  };
  walk(tree, [tree]);
  return tokens;
}

/** Map every UTF-8 byte offset of `text` to its UTF-16 offset. */
function byteToUtf16(text) {
  const map = [];
  let utf16 = 0;
  for (const char of text) {
    const code = char.codePointAt(0);
    const bytes = code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
    for (let i = 0; i < bytes; i++) map.push(utf16);
    utf16 += char.length;
  }
  map.push(utf16);
  return (byte) => map[Math.min(byte, map.length - 1)];
}

/** Map a token to a CSS class, using its own kind and its parents'. */
function tokenClass(token) {
  const { kind } = token;
  const parent = token.ancestors.at(-1).kind;
  if (kind === "COMMENT") return "comment";
  if (kind === "STRING") return "string";
  if (kind === "INTEGER" || kind === "FLOAT") return "number";
  if (kind.startsWith("KW_") || kind.endsWith("_KW")) return "keyword";
  if (parent === "PARAMETER" || kind === "DOLLAR") return "parameter";
  if (parent === "SYMBOLIC_NAME") {
    const owner = token.ancestors.at(-2)?.kind;
    return owner === "LABEL_NAME" || owner === "REL_TYPE_NAME" ? "label" : "name";
  }
  return null;
}

/**
 * Re-render the highlighted copy of the source behind the textarea, with
 * error underlines and the current hover/caret marks. Diagnostics carry
 * UTF-16 `range`s, which line up with the token offsets.
 */
function paintEditor() {
  const { text, tokens, errors } = state;
  const errorMarks = errors.map((e) => ({
    start: e.range.start,
    // Zero-width spans (e.g. "unexpected end of input") still get one cell.
    end: Math.max(e.range.end, e.range.start + 1),
    message: e.message,
  }));
  const marks = hoverMarks.length ? hoverMarks : caretMarks;
  const overlaps = (m, t) => m.start < t.end && m.end > t.start;

  const nodes = tokens.map((token) => {
    const classes = [tokenClass(token)];
    const error = errorMarks.find((m) => overlaps(m, token));
    if (error) classes.push("error");
    for (const mark of marks) if (overlaps(mark, token)) classes.push(mark.className);
    const className = classes.filter(Boolean).join(" ");
    if (!className) return token.text;

    const span = document.createElement("span");
    span.textContent = token.text;
    span.className = className;
    if (error) span.title = error.message;
    return span;
  });

  // An error at end of input has no token to attach to.
  if (errorMarks.some((m) => m.start >= text.length)) {
    const span = document.createElement("span");
    span.className = "error eof";
    span.textContent = " ";
    nodes.push(span);
  }
  highlight.replaceChildren(...nodes, "\n");
}

/** Highlight `marks` while the pointer is over `element`. */
function hoverable(element, marks) {
  element.addEventListener("mouseenter", () => {
    hoverMarks = typeof marks === "function" ? marks() : marks;
    paintEditor();
  });
  element.addEventListener("mouseleave", () => {
    hoverMarks = [];
    paintEditor();
  });
  return element;
}

function select(range) {
  source.focus();
  source.setSelectionRange(range.start, range.end);
}

function onCaretMove() {
  caretMarks = variableMarksAt(source.selectionStart);
  if (!hoverMarks.length) paintEditor();
  if (view === "ast") revealAstNode(source.selectionStart);
}

/** The definition and uses of the variable whose occurrence touches `offset`. */
function variableMarksAt(offset) {
  const variable = state?.analysis?.variables.find((v) =>
    occurrences(v).some((r) => r.start <= offset && offset <= r.end),
  );
  return variable ? variableMarks(variable) : [];
}

function occurrences(variable) {
  return [variable.definition, ...variable.uses];
}

function variableMarks(variable) {
  return [
    { ...variable.definition, className: "mark-def" },
    ...variable.uses.map((r) => ({ ...r, className: "mark-ref" })),
  ];
}

function renderDiagnostic(error) {
  const item = document.createElement("li");
  const position = document.createElement("button");
  position.textContent = `${error.line}:${error.column}`;
  position.title = "Jump to error";
  position.addEventListener("click", () => select(error.range));
  item.append(position, ` ${error.message}`);
  for (const note of error.notes) {
    const detail = document.createElement("div");
    detail.className = `note ${note.level}`;
    detail.textContent = `${note.level}: ${note.message}`;
    item.append(detail);
  }
  return item;
}

// ---------------------------------------------------------------------------
// Output pane
// ---------------------------------------------------------------------------

function renderView() {
  const render = { ast: renderAst, symbols: renderSymbols, params: renderParams, graph: renderGraph, cst: renderCstView, format: renderFormat }[view];
  output.replaceChildren(...render());
}

function el(tag, props = {}, ...children) {
  const element = Object.assign(document.createElement(tag), props);
  element.append(...children.filter((c) => c !== null && c !== undefined));
  return element;
}

function pre(text) {
  return el("pre", { className: "code" }, text);
}

function notice(text) {
  return el("p", { className: "notice" }, text);
}

// --- AST --------------------------------------------------------------------

function renderAst() {
  const toggle = el("input", { type: "checkbox", checked: showRawAst });
  toggle.addEventListener("change", () => {
    showRawAst = toggle.checked;
    renderView();
  });
  const toolbar = el("label", { className: "toolbar" }, toggle, " raw JSON");

  if (!state.ast) return [toolbar, notice("Fix the syntax errors to see the AST.")];
  if (showRawAst) return [toolbar, pre(JSON.stringify(state.ast, null, 2))];

  astNodes = [];
  const tree = el("div", { className: "ast" }, astNode("Query", state.ast, "", 0));
  return [toolbar, tree];
}

const isObject = (value) => value !== null && typeof value === "object";

/**
 * Render one AST value. Enum variants are externally tagged (`{ "Match": {...} }`),
 * so a single capitalised key is shown as the node's name.
 */
function astNode(label, value, path, depth) {
  while (isObject(value) && !Array.isArray(value)) {
    const keys = Object.keys(value);
    if (keys.length !== 1 || !/^[A-Z]/.test(keys[0])) break;
    label = label ? `${label}: ${keys[0]}` : keys[0];
    value = value[keys[0]];
  }

  if (!isObject(value)) {
    return el("div", { className: "ast-leaf" }, el("span", { className: "ast-key" }, label), ` ${JSON.stringify(value)}`);
  }

  const entries = (Array.isArray(value) ? value.map((v, i) => [String(i), v]) : Object.entries(value)).filter(
    ([key, v]) => key !== "span" && v !== null && !(Array.isArray(v) && v.length === 0),
  );

  // Identifiers (`{ name: "a", span }`) read best on one line.
  if (typeof value.name === "string" && entries.length === 1) {
    const leaf = el("div", { className: "ast-leaf" }, el("span", { className: "ast-key" }, label), ` ${value.name}`);
    return withSpan(leaf, value.span, leaf);
  }

  const summary = el("summary", {}, el("span", { className: "ast-key" }, label));
  if (Array.isArray(value)) summary.append(el("span", { className: "ast-meta" }, ` [${value.length}]`));
  if (value.span) summary.append(el("span", { className: "ast-meta" }, ` ${value.span.start}..${value.span.end}`));

  const details = el("details", {}, summary, ...entries.map(([key, v]) => astNode(key, v, `${path}/${key}`, depth + 1)));
  details.open = astOpen.get(path) ?? depth < 4;
  details.addEventListener("toggle", () => astOpen.set(path, details.open));
  return withSpan(details, value.span, summary);
}

/** Link a tree node to its source span: hover highlights, click selects. */
function withSpan(element, span, handle) {
  if (!span) return element;
  const range = { start: state.utf16(span.start), end: state.utf16(span.end) };
  astNodes.push({ element, handle, ...range });
  hoverable(handle, [{ ...range, className: "mark-hover" }]);
  // Clicking the name selects the source; the rest of a summary still toggles.
  const target = handle.tagName === "SUMMARY" ? handle.querySelector(".ast-key") : handle;
  target.classList.add("selectable");
  target.addEventListener("click", (event) => {
    event.preventDefault();
    select(range);
  });
  return element;
}

/** Open and scroll to the innermost tree node containing `offset`. */
function revealAstNode(offset) {
  let best = null;
  for (const node of astNodes) {
    if (node.start <= offset && offset < node.end && (!best || node.end - node.start <= best.end - best.start)) best = node;
  }
  output.querySelector(".ast-current")?.classList.remove("ast-current");
  if (!best) return;
  for (let d = best.element.parentElement?.closest("details"); d; d = d.parentElement?.closest("details")) d.open = true;
  best.handle.classList.add("ast-current");
  best.handle.scrollIntoView({ block: "nearest" });
}

// --- Symbols ----------------------------------------------------------------

/** CST node kinds that name things, and how the Symbols tab titles them. */
const NAME_KINDS = {
  LABEL_NAME: "Labels",
  REL_TYPE_NAME: "Relationship types",
  PROPERTY_KEY_NAME: "Property keys",
  FUNCTION_NAME: "Functions",
};

/**
 * Find every CST node of one of `kinds` and group them by kind and text,
 * e.g. all `LABEL_NAME`s spelling `Person`.
 */
function namedOccurrences(kinds) {
  const owners = new Map();
  for (const token of state.tokens) {
    if (token.kind === "WHITESPACE" || token.kind === "COMMENT") continue;
    const owner = token.ancestors.findLast((a) => kinds.includes(a.kind));
    if (!owner) continue;
    if (!owners.has(owner)) owners.set(owner, []);
    owners.get(owner).push(token);
  }

  const groups = new Map();
  for (const [owner, parts] of owners) {
    const name = parts.map((t) => t.text).join("");
    const key = `${owner.kind}\u0000${name}`;
    if (!groups.has(key)) groups.set(key, { kind: owner.kind, name, ranges: [] });
    groups.get(key).ranges.push({ start: parts[0].start, end: parts.at(-1).end });
  }
  return [...groups.values()];
}

function renderSymbols() {
  const sections = [];

  if (state.analysis) {
    const list = el("ul", { className: "symbols" }, ...state.analysis.variables.map(renderVariable));
    sections.push(el("h3", {}, "Variables"), state.analysis.variables.length ? list : notice("No variables."));
  } else {
    sections.push(el("h3", {}, "Variables"), notice("Fix the errors to resolve variables."));
  }

  const names = namedOccurrences(Object.keys(NAME_KINDS));
  for (const [kind, title] of Object.entries(NAME_KINDS)) {
    const group = names.filter((n) => n.kind === kind);
    if (!group.length) continue;
    const chips = group.map((n) =>
      hoverable(el("button", { className: "chip", title: "Select first occurrence", onclick: () => select(n.ranges[0]) }, n.name, el("span", { className: "count" }, String(n.ranges.length))),
        n.ranges.map((r) => ({ ...r, className: "mark-ref" }))),
    );
    sections.push(el("h3", {}, title), el("div", { className: "chips" }, ...chips));
  }
  return sections;
}

function renderVariable(variable) {
  const name = el("button", { className: "var-name", title: "Select definition", onclick: () => select(variable.definition) }, variable.name);
  const meta = el("span", { className: "var-meta" }, `${variable.kind} · ${variable.uses.length} use${variable.uses.length === 1 ? "" : "s"}`);
  const rename = el("button", { className: "rename" }, "Rename");
  const item = hoverable(el("li", {}, name, meta, rename), variableMarks(variable));

  rename.addEventListener("click", () => {
    const input = el("input", { value: variable.name, spellcheck: false, ariaLabel: `New name for ${variable.name}` });
    const done = (apply) => {
      if (!input.isConnected) return;
      if (apply && input.value && input.value !== variable.name) renameVariable(variable, input.value);
      else input.replaceWith(name);
    };
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") done(true);
      if (event.key === "Escape") done(false);
    });
    input.addEventListener("blur", () => done(false));
    name.replaceWith(input);
    input.select();
  });
  return item;
}

/** Rewrite every occurrence of `variable`, back to front so offsets hold. */
function renameVariable(variable, newName) {
  const replacement = /^[A-Za-z_][A-Za-z0-9_]*$/.test(newName) ? newName : `\`${newName.replaceAll("`", "``")}\``;
  const ranges = occurrences(variable).sort((a, b) => b.start - a.start);
  let text = source.value;
  for (const { start, end } of ranges) text = text.slice(0, start) + replacement + text.slice(end);
  source.value = text;
  update();
}

// --- Parameters -------------------------------------------------------------

function parameters() {
  const byName = new Map();
  for (const group of namedOccurrences(["PARAMETER"])) {
    const name = group.name.replace(/^\$/, "").replace(/^`(.*)`$/s, (_, inner) => inner.replaceAll("``", "`"));
    if (!byName.has(name)) byName.set(name, []);
    byName.get(name).push(...group.ranges);
  }
  return byName;
}

function paramValue(raw) {
  if (raw.trim() === "") return null;
  try {
    return JSON.parse(raw);
  } catch {
    return raw;
  }
}

function renderParams() {
  const params = parameters();
  if (!params.size) return [notice("The query has no $parameters.")];

  const snippet = pre("");
  const refresh = () => {
    const values = Object.fromEntries([...params.keys()].map((name) => [name, paramValue(paramValues.get(name) ?? "")]));
    // The canonical form drops comments; fall back to the raw text while invalid.
    const formatted = format(state.text);
    const cypher = formatted.ok ? formatted.cypher.replace(/;$/, "") : state.text.trim();
    const query = cypher.replace(/[\\`]|\$\{/g, (m) => `\\${m}`);
    snippet.textContent = [
      'import neo4j from "neo4j-driver";',
      "",
      'const driver = neo4j.driver("neo4j://localhost:7687", neo4j.auth.basic("neo4j", "password"));',
      "const { records } = await driver.executeQuery(",
      `  \`${query}\`,`,
      `  ${JSON.stringify(values, null, 2).replaceAll("\n", "\n  ")},`,
      ");",
      "await driver.close();",
    ].join("\n");
  };

  const rows = [...params].map(([name, ranges]) => {
    const input = el("input", { value: paramValues.get(name) ?? "", placeholder: "JSON, or plain text", spellcheck: false });
    input.addEventListener("input", () => {
      paramValues.set(name, input.value);
      refresh();
    });
    const label = el("label", { className: "param" }, el("span", { className: "param-name" }, `$${name}`), input);
    return hoverable(label, ranges.map((r) => ({ ...r, className: "mark-ref" })));
  });

  const copy = el("button", { className: "chip" }, "Copy");
  copy.addEventListener("click", async () => {
    await navigator.clipboard.writeText(snippet.textContent);
    copy.textContent = "Copied";
    setTimeout(() => (copy.textContent = "Copy"), 1200);
  });

  refresh();
  return [el("div", { className: "params" }, ...rows), el("h3", {}, "JavaScript driver ", copy), snippet];
}

// --- Graph ------------------------------------------------------------------

const SVG = "http://www.w3.org/2000/svg";

function svg(tag, attrs = {}, ...children) {
  const element = document.createElementNS(SVG, tag);
  for (const [key, value] of Object.entries(attrs)) element.setAttribute(key, value);
  element.append(...children);
  return element;
}

/** Merge all patterns into one graph; named nodes are shared across clauses. */
function buildGraph(patterns) {
  const nodes = new Map();
  const edges = [];
  let anonymous = 0;
  for (const pattern of patterns) {
    const optional = pattern.clause === "OPTIONAL MATCH";
    const keys = pattern.nodes.map((node) => {
      const key = node.variable ?? `#${anonymous++}`;
      const entry = nodes.get(key) ?? { key, variable: node.variable, labels: new Set(), optional };
      node.labels.forEach((l) => entry.labels.add(l));
      entry.optional &&= optional;
      nodes.set(key, entry);
      return key;
    });
    for (const rel of pattern.relationships) {
      edges.push({ ...rel, from: keys[rel.from], to: keys[rel.to], optional });
    }
  }
  return { nodes: [...nodes.values()], edges };
}

function renderGraph() {
  if (!state.analysis) return [notice("Fix the errors to see the pattern graph.")];
  const { nodes, edges } = buildGraph(state.analysis.patterns);
  if (!nodes.length) return [notice("The query has no MATCH, CREATE or MERGE patterns.")];

  const width = 560;
  const height = 420;
  const radius = 30;
  // shortcut: circle layout, no force simulation; upgrade if queries get large.
  const ring = nodes.length === 1 ? 0 : Math.min(width, height) / 2 - radius - 70; // room for self-loops
  const position = new Map(
    nodes.map((node, i) => {
      const angle = (2 * Math.PI * i) / nodes.length - Math.PI / 2;
      return [node.key, { x: width / 2 + ring * Math.cos(angle), y: height / 2 + ring * Math.sin(angle) }];
    }),
  );

  const marksFor = (name) => () => state.analysis.variables.filter((v) => v.name === name).flatMap(variableMarks);
  const graph = svg("svg", { viewBox: `0 0 ${width} ${height}`, class: "graph", role: "img", "aria-label": "Pattern graph" });
  graph.append(
    svg("defs", {}, svg("marker", { id: "arrow", viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: "auto-start-reverse" }, svg("path", { d: "M0,0 L10,5 L0,10 z" }))),
  );

  // Edges between the same pair of nodes fan out as curves.
  const pairCount = new Map();
  for (const edge of edges) {
    const pair = [edge.from, edge.to].sort().join("\u0000");
    const index = pairCount.get(pair) ?? 0;
    pairCount.set(pair, index + 1);

    const a = position.get(edge.from);
    const b = position.get(edge.to);
    let d;
    let labelAt;
    if (edge.from === edge.to) {
      const r = radius;
      d = `M ${a.x - r * 0.6} ${a.y - r * 0.8} C ${a.x - r * 2} ${a.y - r * 3.2 - index * 20}, ${a.x + r * 2} ${a.y - r * 3.2 - index * 20}, ${a.x + r * 0.6} ${a.y - r * 0.8}`;
      labelAt = { x: a.x, y: a.y - r * 2.6 - index * 15 };
    } else {
      const bend = (index % 2 ? -1 : 1) * Math.ceil(index / 2) * 56;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const length = Math.hypot(dx, dy);
      const control = { x: (a.x + b.x) / 2 - (dy / length) * bend, y: (a.y + b.y) / 2 + (dx / length) * bend };
      const trim = (p, toward) => {
        const tx = toward.x - p.x;
        const ty = toward.y - p.y;
        const l = Math.hypot(tx, ty);
        return { x: p.x + (tx / l) * radius, y: p.y + (ty / l) * radius };
      };
      const start = trim(a, control);
      const end = trim(b, control);
      d = `M ${start.x} ${start.y} Q ${control.x} ${control.y} ${end.x} ${end.y}`;
      labelAt = { x: (start.x + 2 * control.x + end.x) / 4, y: (start.y + 2 * control.y + end.y) / 4 };
    }

    const path = svg("path", { d, class: `edge${edge.variableLength ? " variable-length" : ""}${edge.optional ? " optional" : ""}` });
    if (edge.direction === "right" || edge.direction === "both") path.setAttribute("marker-end", "url(#arrow)");
    if (edge.direction === "left" || edge.direction === "both") path.setAttribute("marker-start", "url(#arrow)");
    const text = [edge.variable, ...edge.types.map((t) => `:${t}`)].filter(Boolean).join(" ") + (edge.variableLength ? " *" : "");
    // A wide transparent copy of the edge makes it easy to hover.
    const hit = svg("path", { d, class: "edge-hit" });
    const group = svg("g", { class: "edge-group" }, hit, path, svg("text", { x: labelAt.x, y: labelAt.y - 4, class: "edge-label" }, text));
    if (edge.variable) hoverable(group, marksFor(edge.variable));
    graph.append(group);
  }

  for (const node of nodes) {
    const { x, y } = position.get(node.key);
    const labels = [...node.labels].map((l) => `:${l}`).join("");
    const group = svg(
      "g",
      { class: `node${node.optional ? " optional" : ""}` },
      svg("circle", { cx: x, cy: y, r: radius }),
      svg("text", { x, y: labels ? y - 2 : y + 4, class: "node-name" }, node.variable ?? "()"),
      svg("text", { x, y: y + 13, class: "node-labels" }, labels),
    );
    if (node.variable) hoverable(group, marksFor(node.variable));
    graph.append(group);
  }

  const legend = notice("Dashed: variable length · faded: OPTIONAL MATCH · hover to find in the query");
  return [graph, legend];
}

// --- CST / formatted --------------------------------------------------------

function renderCstView() {
  return [pre(renderCst(state.tree))];
}

function renderCst(node, depth = 0) {
  const indent = "  ".repeat(depth);
  const line = `${indent}${node.kind}@${node.span.start}..${node.span.end}`;
  if (!("children" in node)) return `${line} ${JSON.stringify(node.text)}`;
  return [line, ...node.children.map((child) => renderCst(child, depth + 1))].join("\n");
}

function renderFormat() {
  const result = format(state.text);
  return [result.ok ? pre(result.cypher) : notice("Fix the syntax errors to format the query.")];
}

// Last, so every module-level declaration above is initialised.
update();
