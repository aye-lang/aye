# ES module support

The engine originally had no ES module support: `import` was an identifier,
`type="module"` scripts were skipped, and `nomodule` scripts ran.

## Pipeline

- The lexer reserves `import` and `export`. The parser makes an ImportCall
  with a captured URL atom; import is never a callable global.
- Classic browser scripts use the document base URL, including external
  scripts and later callbacks. Modules use their own URL (the final URL
  after a redirect). The CLI sets the base to the script file's URL.
- The import driver in `lib.ore` owns a per-engine module map of evaluation
  promises, including failed evaluations. Static imports, dynamic imports
  and external module script elements all use this map.
- `modules.ore` resolves URL-like specifiers and fetches through `net`, so
  the browser session's cache and request machinery are shared. Missing
  resources, unsuccessful HTTP status codes, non-JavaScript HTTP/data MIME
  types and invalid specifiers reject with TypeError. Parsing rejects with
  SyntaxError; evaluation preserves the thrown value.
- Module code runs in a private strict environment with the existing async
  coroutine driver. Static dependencies precede statements; top-level
  await and dynamic import both suspend through the normal promise queue.
- Export getter closures retain live module slots. Static named imports
  retain internal cells that forward reads through the dependency's
  namespace. Numeric/property fast paths must not expose these cells.
- Namespace objects have null prototypes, sorted export keys,
  Symbol.toStringTag `Module`, live data descriptors and non-extensibility.
  Namespace mutation is rejected (or ignored for sloppy assignment).
- `import.meta` is a distinct, stable object per module, with `url`.
  Browser module elements are deferred; classic `nomodule` elements are
  skipped. CLI `.mjs` entries run as modules.

## Deliberate limits / follow-up work

This is an initial module loader, not complete ES module conformance:

- Cyclic static graphs are not linked as strongly connected components;
  they can leave evaluation promises pending. A graph-instantiation/linking
  phase is required before evaluating cycles.
- Re-exports (`export ... from`, `export *`), exported destructuring and
  import attributes/options are not implemented.
- Default named function/class declarations use the expression path, so
  their declaration name is not additionally introduced in module scope.
- Linking errors such as a missing dependency export are found while
  initializing imports, rather than before evaluating the entire graph.
- Import maps (`imports` and `scopes`) are not implemented; bare specifiers
  reject. They require document-level map ingestion, normalization,
  prefix/scoped resolution and integration with graph linking.
- This loader uses the existing network layer; it does not add module CORS
  enforcement. Module async/load-event scheduling is still the browser's
  simplified deferred-script scheduling.

Tests: `231_dynamic_import.js`, `232_module_await.js`, the CLI `.mjs`
entry tests, and `jsdom/dynamic_import.html`. The JS goldens are Node 22
output; the DOM fixture also runs in a safe build with GC stress.
