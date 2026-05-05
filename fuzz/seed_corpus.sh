#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TARGETS=(fuzz_parse_cst fuzz_parse_ast fuzz_parse_hir)
SUITES=(suite-1 suite-2 suite-3 suite-4)

for target in "${TARGETS[@]}"; do
    mkdir -p "${PROJECT_ROOT}/fuzz/corpus/${target}"
done

export PROJECT_ROOT
python3 - <<'PY'
import os

project_root = os.environ["PROJECT_ROOT"]
targets = ["fuzz_parse_cst", "fuzz_parse_ast", "fuzz_parse_hir"]
suites = ["suite-1", "suite-2", "suite-3", "suite-4"]

for target in targets:
    corpus_dir = os.path.join(project_root, "fuzz", "corpus", target)
    for entry in os.listdir(corpus_dir):
        if entry.startswith("seed-"):
            os.remove(os.path.join(corpus_dir, entry))

total = 0
for suite in suites:
    suite_path = os.path.join(project_root, "tests", f"{suite}.cypher")
    with open(suite_path, "r", encoding="utf-8") as f:
        source = f.read()

    # Replicate suite_tests.rs logic exactly:
    # format!("\n{source}").split("\n// ").skip(1)
    blocks = ("\n" + source).split("\n// ")[1:]

    written = 0
    for i, block in enumerate(blocks, start=1):
        try:
            first_newline = block.index("\n")
        except ValueError:
            continue
        query = block[first_newline:].strip()
        if not query:
            continue

        filename = f"seed-{suite}-{i:03d}"
        for target in targets:
            corpus_dir = os.path.join(project_root, "fuzz", "corpus", target)
            filepath = os.path.join(corpus_dir, filename)
            with open(filepath, "wb") as f:
                f.write(query.encode("utf-8"))
        written += 1

    total += written
    print(f"{suite}: wrote {written} queries")

print(f"Seeding complete. {total} queries per target.")
PY
