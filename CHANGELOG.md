# Changelog

All notable changes to `decypher` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate adheres
to [Semantic Versioning](https://semver.org/). While the version is `0.x` /
pre-release, minor and pre-release bumps may contain breaking changes; they
are marked **Breaking** below.

## [Unreleased]

### Added

- WebAssembly bindings in `bindings/wasm`, packaged as the npm package
  `@sunsided/decypher` for Node.js and browsers: `parse` (typed AST as JSON),
  `validate` (syntax errors and unresolved or out-of-scope variables),
  `format`, and `cst` (lossless syntax tree), with TypeScript types and
  runnable Node.js and browser examples.
- The `serde` feature now derives `Serialize` for every AST node and for
  `Span` / `Spanned`; previously it was declared but had no effect.
- `sema::check_aggregation_with` and `sema::analyze_with` take an
  `AggregateRegistry`, so user-defined aggregates count as aggregates in
  semantic analysis too. The registry now lives in `sema` and is re-exported
  as `hir::AggregateRegistry` (#97). `sema::resolve_names_with` takes it as
  well (#105).
- `ErrorKind::AggregateNotAllowed` (with `hir::HirDiagnostic` and
  `sema::SemaError` variants): both analyzers report aggregates outside
  projection items and the `WHERE` / `ORDER BY` of an aggregating projection,
  such as `MATCH (a) WHERE count(a) > 10`, `[x IN l | count(*)]`,
  `CALL p(count(n))` and `count(count(*))` (#103).
- The TCK gate also checks that `sema::analyze` accepts every query the TCK
  expects to compile, and that both analyzers reject queries the TCK expects
  to fail with an undefined variable, an invalid, nested or ambiguous
  aggregation, a column name conflict, a redeclared variable, or one of the
  structural errors below.
- The pattern form of `COUNT { … }`: `COUNT { (n)-->() WHERE … }`, as in
  Neo4j 5 (#107).
- Structural checks in both analyzers, with new `ErrorKind`s (#109):
  `UnionColumnMismatch` (`RETURN 1 AS a UNION RETURN 2 AS b`),
  `InvalidClauseComposition` (`UNION` mixed with `UNION ALL`, updates inside
  `EXISTS` / `COUNT` / `COLLECT`), `InvalidRelationshipPattern` (relationships
  created by `CREATE` / `MERGE` without exactly one type or with a variable
  length, undirected ones in `CREATE`, a relationship matched twice),
  `InvalidDeleteTarget` (`DELETE n:Person`), `MissingAlias`
  (`WITH a, count(*)`) and `NoVariablesInScope` (`MATCH () RETURN *`).
- Checks that need no type or function catalog, in both analyzers (#110):
  `InvalidSkipLimit` (`SKIP n.count`, `LIMIT -1`, `LIMIT 1.5`),
  `InvalidParameterUse` (`MATCH (n $param)`; `CREATE` still allows it),
  `NonDeterministicAggregate` (`count(rand())`) and `VariableTypeConflict`
  with the new `VariableKind` (`MATCH (r) MATCH ()-[r]-()`,
  `WITH 1 AS n MATCH (n)`).

### Changed

- **Breaking:** `ast::CountSubqueryExpression` holds an
  `inner: Box<ExistsInner>` instead of `query`, so it can carry the pattern
  form (#107).
- **Breaking:** `hir::HirDiagnostic::InvalidRelationshipPattern` gains a
  `reason`; new variants `InvalidClauseComposition`, `MissingAlias` and
  `NoVariablesInScope`; the never-reported `VariableOutOfScope` and
  `InvalidMergePattern` are removed (#109).
- **Breaking:** new `hir::HirDiagnostic` variants `InvalidSkipLimit`,
  `InvalidParameterUse`, `NonDeterministicAggregate` and
  `VariableTypeConflict` (#110).
- Both analyzers report redeclared path variables (`WITH 1 AS p MATCH p =
  …`, `MATCH p = (p)-->()`) and `CREATE` / `MERGE` patterns that re-create a
  bound node or relationship (`MATCH (a) CREATE (a)`,
  `MATCH ()-[r]->() CREATE ()-[r:T]->()`) (#108).

- **Breaking:** `analyze` scopes variables per query part: after a `WITH`
  only its columns are visible (`MATCH (p)-->(m) WITH p RETURN m` is
  rejected), and `WITH n.name` without alias no longer binds `name`. The
  `WHERE` and `ORDER BY` of a `WITH` still see the variables it read unless
  it aggregates (or, for `ORDER BY`, is `DISTINCT`), as the openCypher TCK
  requires. `RETURN … ORDER BY` follows the same rule. Each
  `hir::QueryPart::output_scope` is now the scope holding the part's columns
  (#96).
- **Breaking:** `analyze` reports `ErrorKind::AggregationMix` for an
  aggregating projection item that uses variables or properties which are
  not grouping keys (`RETURN n.x + count(n)`), consulting the
  `AggregateRegistry` of the `LowerConfig`. `hir::HirDiagnostic::InvalidAggregation`
  gains a `non_grouping` field (#98).
- `sema::check_aggregation` follows the same grouping rule: every
  non-aggregate item is a grouping key, whatever its expression, so
  `WITH toUpper(n.name) AS k, count(*) AS c` is accepted, while
  `RETURN n.a + n.b, n.a + n.b + count(*)` is rejected as in Neo4j (#97).
- Both analyzers apply the grouping rule to the aggregating `ORDER BY`
  items of an aggregating projection too, so
  `RETURN a + b, count(*) ORDER BY a + b + count(*)`-style queries are
  rejected (#103).
- `analyze` reports a redeclaration when `UNWIND`, `YIELD`, `LOAD CSV`,
  `FOREACH` or a `CALL { … }` column rebinds a visible name.
- **Breaking:** `hir::HirDiagnostic` gains `AggregateNotAllowed`.
- `sema::analyze` lets the `WHERE` and `ORDER BY` of a `WITH` (and the
  `ORDER BY` of a `RETURN`) see the variables the projection read, matching
  `analyze` and the TCK; it rejected 43 valid TCK queries before (#105).
- `analyze` reports unbound variables in pattern predicates
  (`WHERE (n)-[r]->(a)`), which it bound silently and leaked into later
  clauses (#104).
- The wasm `validate()` relies on HIR lowering alone and now also reports
  invalid aggregations.

### Fixed

- `RETURN *, n.x AS x` dropped the explicit items in HIR.
- Variables passed through by `WITH *` / `RETURN *` are grouping keys: the
  `Aggregate` of `RETURN DISTINCT *, count(*)` grouped by nothing, and
  `WITH *, n.x + count(*)` was rejected.
- `task wasm:test` ran no Node.js tests (#106).
- `UNION` chains inside subqueries: `CALL { … }` kept only the first and
  last of three or more branches, and `EXISTS` / `COUNT` / `COLLECT` bodies
  with `UNION` failed to parse. The printer no longer writes a double space
  after a subquery part that ends without `RETURN`
  (`EXISTS { MATCH (n)-->(m) }`).

## [0.2.0-alpha.8] - 2026-10-18

### Added

- GQL / Neo4j 5 syntax: `UNION DISTINCT`, and `IS` label predicates in
  expressions and patterns (`WHERE n IS Person`, `(n IS Person)`,
  `[r IS KNOWS]`) (#90, #69).
- The GQL wildcard label `%` in label and relationship-type expressions
  (`(n:%)`, `(n:!%)`, `[r:%]`, `n IS %`) (#94, #91).
- Label expressions beyond static labels in HIR: new `hir::LabelExpr`, used
  by `ExprKind::LabelCheck`, `NodePattern::label_expression` and
  `RelationshipPattern::type_expression`. `n:A|B`, `(n:!A)`, `[r:!T]` and
  `$(…)` labels no longer fail `analyze` as unsupported; `(n:A&B)` lowers to
  two required labels (#94, #89).
- GQL numeric type names in type predicates (`UNSIGNED INTEGER`,
  `INT8`…`UINT256`, `BIGINT`, `DECIMAL`, `REAL`, `DOUBLE PRECISION`) (#93).
- openCypher TCK conformance gate (`tests/tck.rs`): every query of the
  openCypher 2024.3 TCK syntax projection is checked for accept/reject,
  print/re-parse stability, lossless printing and, where the TCK expects no
  error, semantic analysis. Known gaps are listed in
  `tests/tck/known_failures.txt`, which may only shrink (#75).
- Projection items can wrap aggregates in larger expressions:
  `RETURN abs(sum(n.x))`, `RETURN count(n) + 1`,
  `RETURN reduce(s = 0, x IN collect(n.v) | s + x)`. HIR computes the
  aggregate calls in `Aggregate` and builds the surrounding expression in a
  following `Project` (#85, #68).
- Parsing of octal (`0o17`), leading-dot floats (`.5`) and
  `-9223372036854775808` (#81, #63); lists starting with `-` or `(`
  (`[-1, -2]`, `[(1 + 2) * 3]`) (#81).
- Non-reserved keywords as names in expressions (`rows`, `node`, `types`,
  `match`, `key`), and `null`/`true`/`false` as map keys (#82, #64).
- Pattern property parameters (`(n $props)`, `[r:T $props]`), legacy
  relationship-type alternation (`[:A|:B]`), parenthesised SET targets
  (`SET (n).name = …`) and keyword labels in label predicates
  (`RETURN m:TYPE`) (#82, #65).
- Pattern expressions in expression position (`WHERE (a)-->(b)`,
  `size((a)-->())`) and real pattern comprehensions (#76, #61, #62).
- `CALL { … }` subqueries see the enclosing variables through their
  importing `WITH` and export their returned columns; new
  `hir::ops::CallSubqueryOp::exported_bindings` (#83, #67).

### Changed

- **Breaking:** new `ast::Expression::IsNormalized` / `IsTyped` (with
  `ast::expr::NormalForm`), `ast::LabelExpression::Wildcard`, and
  `hir::ExprKind::IsNormalized` / `IsTyped` / `LabelCheck`; new fields
  `hir::NodePattern::label_expression` and
  `hir::RelationshipPattern::type_expression` (#92, #94).
- **Breaking:** `ast::ProcedureInvocation` gains `implicit_arguments`;
  `ast::MapProjectionItem` gains a `Variable` variant; `ast::LoadCsv` and
  `hir::ops::LoadCsvOp` gain `field_terminator`;
  `hir::ops::CallSubqueryOp` gains `exported_bindings` (#73, #80, #83).
- **Breaking:** an escaped parameter such as `` $`odd param` `` is now named
  `odd param`, without backticks (#80).
- **Breaking:** HIR reuses the existing binding when a pattern mentions an
  already bound variable, instead of allocating a new one per occurrence
  (#83, #79).
- In-query procedure calls require an explicit argument list and named
  `YIELD` items; `CALL p YIELD x RETURN x` and `… YIELD * RETURN …` are
  rejected as openCypher requires (#73).
- Aliases are no longer visible to other items of the same `WITH`/`RETURN`;
  `WITH 1 AS a, a + 1 AS b` is rejected by both `analyze` and
  `sema::analyze` (#85).
- Overflowing and malformed number literals report
  `ErrorKind::InvalidNumber` instead of an internal error; a missing `RETURN`
  reports `ErrorKind::MissingClause` (#81, #54).

### Fixed

- `IS [NOT] [NFC|NFD|NFKC|NFKD] NORMALIZED` and `IS [NOT] :: <type>` /
  `IS TYPED <type>` were rebuilt as `IS NULL`, silently changing the query;
  they are now kept as their own predicates (#92, #88).
- Queries that parsed but silently changed meaning: map values truncated to
  their first operand, list elements and `DELETE` targets duplicated from
  flat expression chains, `UNWIND` binding the wrong variable, parenthesised
  function arguments dropped, in-query `CALL … YIELD` dropped,
  `SKIP`/`LIMIT`/`LOAD CSV`/map-projection expressions truncated
  (#73, #56–#60).
- `SET n:A:B` / `REMOVE n:A` losing their variable and extra labels,
  `exists { pattern WHERE … }` printing empty, map projection variable
  selectors turning into property selectors, `FIELDTERMINATOR` dropped
  (#80, #74, #77, #78).
- Printer: missing `CALL`/`YIELD` keywords, doubled backticks on escaped
  parameters and identifiers, `$1` printed as `` $`1` `` (#73, #80, #66).
- `MERGE p = …` path variables bound (#83, #67).
- Subquery bodies (`CALL`, `EXISTS`, `COUNT`, `COLLECT`) are analysed with the
  enclosing scope and report their errors instead of silently lowering to an
  empty query (#83, #67).
- `sema::analyze`: CREATE/MERGE patterns bound, repeated pattern variables and
  self-loops (`MATCH (a)-[r]->(a)`) accepted, UNION branches and `RETURN`
  aliases scoped correctly (#83, #85).
- Duplicate projection columns (`RETURN 1 AS a, 2 AS a`,
  `RETURN count(*), count(*)`) reported by both `analyze` and
  `sema::analyze` (#85, #84).

## [0.2.0-alpha.7] - 2026-10-16

### Added

- `reduce(acc = init, x IN list | expr)` as `ast::Expression::Reduce` /
  `hir::ExprKind::Reduce`, with `acc` and `x` scoped to the expression (#54).
- `all`/`any`/`none`/`single(x IN list WHERE …)` build the quantifier
  expressions instead of plain function calls (#54).

### Changed

- **Breaking:** `ast::ListComprehension` gains `collection` (the source list
  was previously dropped); `ast::Expression` and `hir::ExprKind` gain
  `Reduce` (#54).
- A second `RETURN` in one query part is rejected with
  `ErrorKind::DuplicateClause` (#54).
- `rowan` updated to 0.17 (#50).

### Fixed

- Variable references dropped as function arguments, list comprehension
  collections and `FOREACH` lists (`abs(x)` → `abs()`) (#54).
- List comprehension `WHERE` filters, and `IN` inside them (#54).
- `WITH … CREATE … RETURN` losing its updating clauses (#54).
- `range` usable as a function name (#49).
- Broken `ToCypher` intra-doc link (#55).

## Earlier versions

See the [GitHub releases](https://github.com/sunsided/decypher/releases) for
0.2.0-alpha.6 and earlier.

[Unreleased]: https://github.com/sunsided/decypher/compare/v0.2.0-alpha.8...HEAD
[0.2.0-alpha.8]: https://github.com/sunsided/decypher/compare/v0.2.0-alpha.7...v0.2.0-alpha.8
[0.2.0-alpha.7]: https://github.com/sunsided/decypher/compare/v0.2.0-alpha.6...v0.2.0-alpha.7
