# ES modules

Static imports and exports, `import()`, `import.meta`, top-level await,
module scripts, JSON modules and import maps. Before B8 the engine had
none of these: `type="module"` scripts were skipped and `nomodule` ran.

## Pipeline

- **Records.** `modules.ore` fetches a module through `net` (the session's
  cache and requests) and parses it (`parser.moduleProgram`) into a record:
  its requests (specifier and `type` attribute), import entries, local,
  indirect and star exports, and whether it has top-level await. A missing
  resource, an unsuccessful status, a non-JavaScript (or, for
  `type: 'json'`, non-JSON) MIME type over HTTP and an invalid specifier
  are TypeErrors; a parse error is a SyntaxError. Re-exporting an imported
  binding (`import {a}; export {a}`) is an indirect export.
- **Code.** A module's code is one async function run as a coroutine. It
  stops twice before its statements: once instantiated (its environment
  and hoisted functions exist; it hands out a live cell per exported
  local and puts its let/const/class slots in their TDZ) and once linked
  (its import bindings hold the cells it was given).
- **Cells.** A cell (`Class.ModuleBinding`, `ObjData.Cell`) names an
  environment slot. Import bindings are `DECL_IMPORT` consts; the resolver
  marks their identifiers `IDENT_IMPORT`, and only those reads unwrap a
  cell (fast expressions skip them). A namespace import's slot holds the
  namespace itself.
- **Namespaces.** Exotic objects (`Class.ModuleNamespace`): null
  prototype, sorted export names, `Symbol.toStringTag` `Module`, not
  extensible, each export a data property whose value is read through its
  cell (a native getter bound to it, flagged `F_MODULE`). Writes, deletes
  and redefinitions fail (TypeError in strict code); `Object.keys` throws
  for an export in its TDZ.
- **Loader** (`lib.ore`). One module map per engine, keyed by URL (plus
  the type), with failed fetches and parses cached. It loads a whole graph
  first, a level at a time (each level's new modules are prefetched
  concurrently), then links it (InnerModuleLinking: ResolveExport with
  `export *` ambiguity and `default` exclusion, GetExportedNames,
  SyntaxErrors before anything runs), then evaluates it
  (InnerModuleEvaluation over strongly connected components, and the
  ES2022 async module algorithms: AsyncParentModules, pending
  dependencies, execution in async-evaluation order). Evaluation errors
  are cached on the records. Sync graphs run in one job.
- **Entry points.** `import()` (with the `with` option), module scripts
  (`<script type=module>`, external or inline, deferred in document order
  with `defer` scripts; errors before evaluation are reported as uncaught,
  a failed evaluation through `reportError`), the CLI's `.mjs` entries,
  and `import.meta.resolve`. Classic scripts resolve against the document
  base; a module against its own (final) URL.
- **Import maps.** `<script type="importmap">` before the modules: `imports`
  and `scopes` with exact and prefix (`/`) keys, null entries blocking a
  specifier, later maps merged in.

## Limits

- Module scripts inserted by a script (not the parser) do not run, as for
  classic scripts; script elements fire no `load`/`error` events.
- No module CORS checks, integrity, or `import.meta` beyond `url` and
  `resolve`. Import attribute types other than `json` are TypeErrors.
- `async` module scripts run like deferred ones.

Tests: `231`-`237` and `242`-`245` in `browser/tests/js` (Node 22 output; cycles,
re-exports, TLA, errors, JSON), the CLI `.mjs` entries,
`jsdom/dynamic_import.html` and `jsdom/module_scripts.html` (also under
GC stress).
