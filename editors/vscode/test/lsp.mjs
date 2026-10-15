// Spawns `aye lsp` (AYE=<path>, default ../../target/release/aye), sends
// initialize and didOpen for a file with an error, and checks that a
// publishDiagnostics with that error comes back.
import { spawn } from "child_process";
import path from "path";
import { fileURLToPath } from "url";

const here = path.dirname(fileURLToPath(import.meta.url));
const aye = process.env.AYE ?? path.resolve(here, "../../../target/release/aye");
const server = spawn(aye, ["lsp"], { stdio: ["pipe", "pipe", "inherit"] });
const send = (msg) => {
  const body = JSON.stringify({ jsonrpc: "2.0", ...msg });
  server.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
};

const timer = setTimeout(() => {
  console.error("lsp test: no publishDiagnostics within 10 s");
  process.exit(1);
}, 10000);

let buf = Buffer.alloc(0);
server.stdout.on("data", (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  for (;;) {
    const sep = buf.indexOf("\r\n\r\n");
    if (sep < 0) return;
    const len = Number(/Content-Length: (\d+)/i.exec(buf.subarray(0, sep).toString())[1]);
    if (buf.length < sep + 4 + len) return;
    const msg = JSON.parse(buf.subarray(sep + 4, sep + 4 + len).toString());
    buf = buf.subarray(sep + 4 + len);
    if (msg.id === 1) {
      console.log("initialize ->", Object.keys(msg.result.capabilities).join(", "));
      send({ method: "initialized", params: {} });
      const uri = "file://" + path.join(here, "lsp_error.aye");
      send({
        method: "textDocument/didOpen",
        params: { textDocument: { uri, languageId: "aye", version: 1, text: "pub fn main() {\n    let x: i32 = true\n}\n" } },
      });
    } else if (msg.method === "textDocument/publishDiagnostics") {
      const d = msg.params.diagnostics;
      d.forEach((x) => console.log(`diagnostic ${x.code ?? ""} line ${x.range.start.line + 1}: ${x.message}`));
      clearTimeout(timer);
      send({ id: 2, method: "shutdown" });
      send({ method: "exit" });
      if (d.length === 0 || d[0].range.start.line !== 1) {
        console.error("lsp test: expected a diagnostic on line 2");
        process.exit(1);
      }
      console.log("lsp test: ok");
      process.exit(0);
    }
  }
});
send({ id: 1, method: "initialize", params: { processId: process.pid, rootUri: null, capabilities: {} } });
