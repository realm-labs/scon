import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("scon");
  const command = config.get<string>("server.path", "scon-lsp");
  const serverOptions: ServerOptions = { command };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "scon" }],
    synchronize: {
      configurationSection: "scon"
    },
    initializationOptions: getServerSettings()
  };

  client = new LanguageClient("scon", "SCON Language Server", serverOptions, clientOptions);
  context.subscriptions.push(client.start());
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}

function getServerSettings() {
  const config = vscode.workspace.getConfiguration("scon");
  return {
    scon: {
      includeRoot: config.get<string>("includeRoot", ""),
      format: {
        enable: config.get<boolean>("format.enable", true)
      },
      diagnostics: {
        resolveOnChange: config.get<boolean>("diagnostics.resolveOnChange", true)
      },
      maxFileSize: config.get<number>("maxFileSize", 1048576)
    }
  };
}
