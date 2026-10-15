// Starts `aye lsp` (JSON-RPC over stdin and stdout) for .aye files:
// diagnostics, quick fixes, formatting, hover and go to definition.
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

// The `aye` binary: the `aye.path` setting, else `aye` on PATH, else the
// installer's `~/.aye/bin/aye`. Undefined if none of them exists.
function findAye(): string | undefined {
  const configured = vscode.workspace.getConfiguration("aye").get<string>("path", "").trim();
  if (configured) {
    const p = configured.startsWith("~") ? path.join(os.homedir(), configured.slice(1)) : configured;
    if (path.isAbsolute(p)) {
      return fs.existsSync(p) ? p : undefined;
    }
    return onPath(p);
  }
  return onPath("aye") ?? existing(path.join(os.homedir(), ".aye", "bin", exe("aye")));
}

function exe(name: string): string {
  return process.platform === "win32" ? name + ".exe" : name;
}

function existing(p: string): string | undefined {
  return fs.existsSync(p) ? p : undefined;
}

function onPath(name: string): string | undefined {
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    if (dir) {
      const p = existing(path.join(dir, exe(name)));
      if (p) {
        return p;
      }
    }
  }
  return undefined;
}

async function start(context: vscode.ExtensionContext): Promise<void> {
  const aye = findAye();
  if (!aye) {
    const configured = vscode.workspace.getConfiguration("aye").get<string>("path", "");
    const where = configured ? `at "${configured}" (setting aye.path)` : "on PATH or in ~/.aye/bin";
    const pick = await vscode.window.showWarningMessage(
      `Aye: the aye compiler was not found ${where}. Syntax colors work; diagnostics, formatting and hover need it.`,
      "Install Aye",
      "Set aye.path",
    );
    if (pick === "Install Aye") {
      vscode.env.openExternal(vscode.Uri.parse("https://ayelang.dev/#install"));
    } else if (pick === "Set aye.path") {
      vscode.commands.executeCommand("workbench.action.openSettings", "aye.path");
    }
    return;
  }
  const serverOptions: ServerOptions = {
    command: aye,
    args: ["lsp"],
    transport: TransportKind.stdio,
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "aye" }],
  };
  client = new LanguageClient("aye", "Aye", serverOptions, clientOptions);
  await client.start();
}

async function stop(): Promise<void> {
  if (client) {
    const c = client;
    client = undefined;
    await c.stop();
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("aye.restartServer", async () => {
      await stop();
      await start(context);
    }),
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration("aye.path")) {
        await stop();
        await start(context);
      }
    }),
  );
  await start(context);
}

export function deactivate(): Promise<void> {
  return stop();
}
