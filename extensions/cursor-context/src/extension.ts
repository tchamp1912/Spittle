import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

const CONTEXT_DIR = path.join(os.homedir(), "Library", "Caches", "spittle");
const CONTEXT_FILE = path.join(CONTEXT_DIR, "cursor_context.json");

interface SpittleContext {
  workspaceRoots: string[];
  activeFile: string | null;
}

function writeContext(context: SpittleContext): void {
  try {
    fs.mkdirSync(CONTEXT_DIR, { recursive: true });
    fs.writeFileSync(CONTEXT_FILE, JSON.stringify(context, null, 2));
  } catch {
    // Silently ignore write errors
  }
}

function updateContext(): void {
  const workspaceRoots = (vscode.workspace.workspaceFolders ?? []).map(
    (f) => f.uri.fsPath,
  );

  const activeFile =
    vscode.window.activeTextEditor?.document.uri.fsPath ?? null;

  writeContext({ workspaceRoots, activeFile });
}

export function activate(context: vscode.ExtensionContext): void {
  // Write context immediately on activation
  updateContext();

  // Update when workspace folders change
  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(() => updateContext()),
  );

  // Update when active editor changes
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => updateContext()),
  );
}

export function deactivate(): void {
  // Clean up context file on deactivation
  try {
    fs.unlinkSync(CONTEXT_FILE);
  } catch {
    // Ignore
  }
}
