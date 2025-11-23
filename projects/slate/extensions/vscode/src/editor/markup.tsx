// deno-lint-ignore-file
import * as vscode from "vscode";
import { Uri } from "vscode";
import type { CancellationToken } from "vscode";
import type { TextDocument, CustomTextEditorProvider } from "vscode";
import type { Webview, WebviewPanel, WebviewView, WebviewViewProvider, WebviewViewResolveContext } from "vscode";

import { parseMarkup } from "@brainbow/slate/parse/markup";

import { TracingSubscriber } from "../tracing.ts";

const localEmbedAllow = [
    "vscode-webview:",
    "http://localhost",
    "http://127.0.0.1",
];

export function generateScriptKey(resourceAddress?: any) {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}

export function updateTextDocument(document: TextDocument, newContent: string) {
    const edit = new vscode.WorkspaceEdit();

    // Define a range that covers the entire existing document
    const fullRange = new vscode.Range(
        document.lineAt(0).range.start,
        document.lineAt(document.lineCount - 1).range.end
    );

    // Replace the entire content with the new content
    edit.replace(document.uri, fullRange, newContent);

    // Apply the edit to the document
    return vscode.workspace.applyEdit(edit);
}

export async function searchWorkspaceFiles(pattern: string) {
    if (!vscode.workspace.workspaceFolders) {
        return;
    }
    try {
        const fileUris: vscode.Uri[] = await vscode.workspace.findFiles(
            pattern,
            '**/node_modules/**',
            1000
        );

        return fileUris.map(uri => ({
            path: uri.fsPath,
            uri: uri.toString()
        }));
    } catch (error) {
        console.error("File search failed during webview resolution:", error);
    }
}

export class MarkupEditor implements CustomTextEditorProvider {
    private panel?: WebviewPanel;
    private documentWatcher?: any;

    constructor(
        private readonly extensionUri: Uri,
        private readonly tracer: TracingSubscriber,
    ) {
        //..
    }

    resolveCustomTextEditor(document: TextDocument, panel: WebviewPanel, token: CancellationToken) {
        panel.webview.postMessage({
            kind: 'event:onDocumentReady',
            document: document,
        });

        panel.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                this.extensionUri,
            ]
        };

        panel.webview.onDidReceiveMessage((message: any) => {
            switch (message.kind) {
                case "request:showInfoMessage":
                    vscode.window.showInformationMessage(message.summary);
                    break;
                case "request:logDebugMessage":
                    const { content } = message;
                    this.tracer.debug(content);
                    break;
                case "request:openResource":
                    vscode.window.showInformationMessage(message.link);
                    break;
                case 'event:onChange':
                    // TODO: Validate user content!
                    updateTextDocument(document, message.content);
                    break;
                case 'event:onError':
                    const { summary, error } = message;
                    this.tracer.error(`Sketch editor failed: ${summary};`, error);
                    break;
                default:
                    const dbgMessage = JSON.stringify(message, null, 2);
                    this.tracer.debug(`Received unknown '${message.kind}' message:`, dbgMessage);
            }
        });

        this.documentWatcher = vscode.workspace.onDidChangeTextDocument(changeEvent => {
            if (changeEvent.document.uri.toString() === document.uri.toString()) {
                panel.webview.html = this.draw(changeEvent.document, panel);
            }
        });

        panel.webview.html = this.draw(document, panel);

        this.panel = panel;
    }

    public draw(doc: TextDocument, panel: WebviewPanel): string {
        const dbgPanel = JSON.stringify(panel, null, 2);
        this.tracer.debug(`Drawing Markup:`, dbgPanel);

        const assetDir = Uri.joinPath(this.extensionUri, "web/assets");
        const stylesheet = Uri.joinPath(assetDir, "editor/markup.css");
        const editorScript = Uri.joinPath(assetDir, "editor/markup.js");
        const editorScriptKey = generateScriptKey(editorScript);

        const excalidrawAssetsPrefix = "https://cdn.jsdelivr.net/npm/@excalidraw/excalidraw";
        const trustEmbedDomains = [
            ...localEmbedAllow,
            // TODO: Move to registry/settings.
            "http://*.stooper.local",
            "https://*.brainbow.local",
            "https://*.youtube.com",
        ];

        const documentContent = doc.getText();
        // const documentStructure = parseMarkup(documentContent);

        // this.tracer.debug(`Document:`, JSON.stringify(documentStructure, null, 2));

        return `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <title>Markup Editor</title>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta http-equiv="Content-Security-Policy" content="
                    default-src 'self';
                    script-src 'unsafe-inline' 'nonce-${editorScriptKey}';
                    style-src 'unsafe-inline' ${panel.webview.cspSource} ${excalidrawAssetsPrefix};
                    font-src ${panel.webview.cspSource} ${excalidrawAssetsPrefix} data:;
                    img-src ${panel.webview.cspSource} ${excalidrawAssetsPrefix} data:;
                    frame-src 'self' ${trustEmbedDomains.join(' ')};
                    ${panel.webview.cspSource}
                ">
                <link href="${panel.webview.asWebviewUri(stylesheet)}" rel="stylesheet">
                <script src="${panel.webview.asWebviewUri(editorScript)}" nonce="${editorScriptKey}" type="module"></script>
            </head>
            <body>
                <div id="overlay" class="dev">
                    Loading ..
                </div>
                <main id="root" class="document">
                    ${documentContent}
                </main>
            </body>
            </html>
        `
    }
}
