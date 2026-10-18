import init, { cst, format, parse, validate, version } from "@sunsided/decypher";

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

let view = "ast";
for (const tab of tabs) {
  tab.addEventListener("click", () => {
    view = tab.dataset.view;
    tabs.forEach((t) => t.setAttribute("aria-selected", String(t === tab)));
    update();
  });
}

source.addEventListener("input", update);
source.addEventListener("scroll", () => {
  highlight.scrollTop = source.scrollTop;
  highlight.scrollLeft = source.scrollLeft;
});

update();

function update() {
  const text = source.value;

  // The CST is lossless: concatenating its token texts reproduces the input
  // exactly, so it can drive syntax highlighting even while the query is
  // invalid.
  const tree = cst(text);
  const checked = validate(text);
  const errors = checked.ok ? [] : checked.errors;

  highlight.replaceChildren(...renderHighlight(tree.tree, text, errors), "\n");
  diagnostics.replaceChildren(...errors.map(renderDiagnostic));

  if (view === "ast") {
    const result = parse(text);
    output.textContent = result.ok ? JSON.stringify(result.ast, null, 2) : "(syntax errors)";
  } else if (view === "cst") {
    output.textContent = renderCst(tree.tree);
  } else {
    const result = format(text);
    output.textContent = result.ok ? result.cypher : "(syntax errors)";
  }
}

/** Map a token to a CSS class, using its own kind and its parent's. */
function tokenClass(kind, parentKind) {
  if (kind === "COMMENT") return "comment";
  if (kind === "STRING") return "string";
  if (kind === "INTEGER" || kind === "FLOAT") return "number";
  if (kind.startsWith("KW_") || kind.endsWith("_KW")) return "keyword";
  if (parentKind === "PARAMETER" || kind === "DOLLAR") return "parameter";
  if (parentKind === "SYMBOLIC_NAME") return "name";
  return null;
}

/**
 * Turn the CST into highlighted DOM nodes. Diagnostics carry UTF-16 `range`s,
 * which line up with JavaScript string indices, so error underlines are
 * placed by tracking our position in the source while emitting tokens.
 */
function renderHighlight(tree, text, errors) {
  const tokens = [];
  const collect = (node, ancestors) => {
    for (const child of node.children) {
      if ("children" in child) collect(child, [...ancestors, child.kind]);
      else tokens.push({ ...child, parentKind: ancestors.at(-1), grandparentKind: ancestors.at(-2) });
    }
  };
  collect(tree, [tree.kind]);

  const marks = errors.map((e) => ({
    start: e.range.start,
    // Zero-width spans (e.g. "unexpected end of input") still get one cell.
    end: Math.max(e.range.end, e.range.start + 1),
    message: e.message,
  }));

  const nodes = [];
  let offset = 0;
  for (const token of tokens) {
    const start = offset;
    offset += token.text.length;
    const parent = token.parentKind === "SYMBOLIC_NAME" ? token.grandparentKind : token.parentKind;
    let className = tokenClass(token.kind, token.parentKind);
    if (className === "name" && (parent === "LABEL_NAME" || parent === "REL_TYPE_NAME")) {
      className = "label";
    }
    const error = marks.find((m) => m.start < offset && m.end > start);

    if (!className && !error) {
      nodes.push(token.text);
      continue;
    }
    const span = document.createElement("span");
    span.textContent = token.text;
    if (className) span.className = className;
    if (error) {
      span.classList.add("error");
      span.title = error.message;
    }
    nodes.push(span);
  }

  // An error at end of input has no token to attach to.
  if (marks.some((m) => m.start >= text.length)) {
    const span = document.createElement("span");
    span.className = "error eof";
    span.textContent = " ";
    nodes.push(span);
  }
  return nodes;
}

function renderDiagnostic(error) {
  const item = document.createElement("li");
  const position = document.createElement("button");
  position.textContent = `${error.line}:${error.column}`;
  position.title = "Jump to error";
  position.addEventListener("click", () => {
    source.focus();
    source.setSelectionRange(error.range.start, Math.max(error.range.end, error.range.start));
  });
  item.append(position, ` ${error.message}`);
  for (const note of error.notes) {
    const detail = document.createElement("div");
    detail.className = `note ${note.level}`;
    detail.textContent = `${note.level}: ${note.message}`;
    item.append(detail);
  }
  return item;
}

function renderCst(node, depth = 0) {
  const indent = "  ".repeat(depth);
  const line = `${indent}${node.kind}@${node.span.start}..${node.span.end}`;
  if (!("children" in node)) return `${line} ${JSON.stringify(node.text)}`;
  return [line, ...node.children.map((child) => renderCst(child, depth + 1))].join("\n");
}
