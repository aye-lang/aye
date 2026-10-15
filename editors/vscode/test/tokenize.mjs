// Tokenizes every .aye file of the repo (std/, site/samples/, tests/) with
// the grammar, as VS Code does, and fails if a line ends inside a string,
// if a file ends inside a `fn (...)` parameter list, or if a file outside
// tests/ (whose error cases are invalid on purpose) has an `invalid.illegal`
// token.
//   node test/tokenize.mjs [files...]
import fs from "fs";
import path from "path";
import { createRequire } from "module";
import { fileURLToPath } from "url";
import vsctm from "vscode-textmate";
import oniguruma from "vscode-oniguruma";

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../../..");
const grammarPath = path.resolve(here, "../syntaxes/aye.tmLanguage.json");

const wasm = fs.readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm"));
await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
const registry = new vsctm.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (p) => new oniguruma.OnigScanner(p),
    createOnigString: (s) => new oniguruma.OnigString(s),
  }),
  loadGrammar: async (scope) =>
    scope === "source.aye" ? vsctm.parseRawGrammar(fs.readFileSync(grammarPath, "utf8"), grammarPath) : null,
});
const grammar = await registry.loadGrammar("source.aye");

function walk(dir, out) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (e.name.endsWith(".aye")) out.push(p);
  }
  return out;
}

export function tokenize(text) {
  let stack = vsctm.INITIAL;
  const lines = [];
  for (const line of text.split("\n")) {
    const r = grammar.tokenizeLine(line, stack);
    lines.push({ line, tokens: r.tokens, endStack: r.ruleStack });
    stack = r.ruleStack;
  }
  return { lines, endStack: stack };
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const files = process.argv.length > 2
    ? process.argv.slice(2)
    : ["std", "site/samples", "tests"].flatMap((d) => walk(path.join(root, d), []));
  let bad = 0;
  let illegal = 0;
  let lines = 0;
  for (const f of files) {
    const text = fs.readFileSync(f, "utf8");
    const { lines: ls, endStack } = tokenize(text);
    lines += ls.length;
    ls.forEach(({ line, tokens, endStack: s }, i) => {
      const last = tokens[tokens.length - 1];
      if (last && last.scopes.some((x) => x.startsWith("string.quoted.double"))) {
        // An unclosed string ends at `$`, which pops it; still inside one means a spill.
        if (s.depth > 1) {
          console.log(`${f}:${i + 1}: line ends inside a string`);
          bad++;
        }
      }
      for (const t of tokens) {
        if (t.scopes.some((x) => x.startsWith("invalid.illegal"))) {
          illegal++;
          // Only the compiler's error tests may hold invalid code.
          if (!path.relative(root, f).startsWith("tests" + path.sep)) {
            console.log(`${f}:${i + 1}: invalid.illegal in a valid file: \`${line.slice(t.startIndex, t.endIndex)}\``);
            bad++;
          }
          if (process.env.VERBOSE) console.log(`${f}:${i + 1}: illegal \`${line.slice(t.startIndex, t.endIndex)}\``);
        }
      }
    });
    if (endStack.depth > 1) {
      console.log(`${f}: file ends inside ${endStack.depth - 1} open region(s)`);
      bad++;
    }
  }
  console.log(`tokenized ${files.length} files, ${lines} lines: ${bad} spills, ${illegal} invalid.illegal tokens`);
  process.exit(bad ? 1 : 0);
}
