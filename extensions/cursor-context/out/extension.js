"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = require("vscode");
const fs = require("fs");
const path = require("path");
const os = require("os");
const CONTEXT_DIR = path.join(os.homedir(), "Library", "Caches", "spittle");
const CONTEXT_FILE = path.join(CONTEXT_DIR, "cursor_context.json");
function writeContext(context) {
  try {
    fs.mkdirSync(CONTEXT_DIR, { recursive: true });
    fs.writeFileSync(CONTEXT_FILE, JSON.stringify(context, null, 2));
  } catch {
    // Silently ignore write errors
  }
}
function updateContext() {
  const workspaceRoots = (vscode.workspace.workspaceFolders ?? []).map(
    (f) => f.uri.fsPath,
  );
  const activeFile =
    vscode.window.activeTextEditor?.document.uri.fsPath ?? null;
  writeContext({ workspaceRoots, activeFile });
}
function activate(context) {
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
function deactivate() {
  // Clean up context file on deactivation
  try {
    fs.unlinkSync(CONTEXT_FILE);
  } catch {
    // Ignore
  }
}
//# sourceMappingURL=extension.js.map
