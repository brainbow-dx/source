// deno-lint-ignore-file
import * as vscode from "vscode";
import { Uri } from "vscode";
import type { CancellationToken } from "vscode";
import type { TextDocument, CustomTextEditorProvider } from "vscode";
import type { Webview, WebviewPanel, WebviewView, WebviewViewProvider, WebviewViewResolveContext } from "vscode";

// import { parseMarkup } from "@escher/sdk/parse/markup";

import { TracingSubscriber } from "../tracing.ts";

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

export class SketchEditor implements CustomTextEditorProvider {
    private panel?: WebviewPanel;
    private documentWatcher?: any;

    constructor(
        private readonly extensionUri: Uri,
        private readonly tracer: TracingSubscriber,
    ) {
        //..
    }

    resolveCustomTextEditor(document: TextDocument, panel: WebviewPanel, token: CancellationToken) {
        this.documentWatcher = vscode.workspace.onDidChangeTextDocument(changeEvent => {
            // 2. Filter the event to only process changes for the current document
            if (changeEvent.document.uri.toString() === document.uri.toString()) {
                panel.webview.postMessage({
                    kind: 'event:onDocumentUpdated',
                    document: document,
                });
            }
        });

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

        panel.webview.html = this.draw(document, panel);

        this.panel = panel;
    }

    public draw(doc: TextDocument, panel: WebviewPanel): string {
        const dbgPanel = JSON.stringify(panel, null, 2);
        this.tracer.debug(`Drawing Sketch:`, dbgPanel);

        const assetDir = Uri.joinPath(this.extensionUri, "web/assets");

        const stylesheet = Uri.joinPath(assetDir, "editor/sketch.css");

        const editorScript = Uri.joinPath(assetDir, "editor/sketch.js");
        const editorScriptKey = generateScriptKey(editorScript);

        // TODO: Move to registry/settings.
        const trustEmbedDomains = [
            "vscode-webview:",
            "http://localhost",
            "http://127.0.0.1",
            "http://*.stooper.localhost",
            "https://*.brainbow.localhost",
            "https://www.youtube.com",
            "https://cdn.jsdelivr.net/npm/@excalidraw/excalidraw",
        ];

        return `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <title>Sketch Editor</title>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta http-equiv="Content-Security-Policy" content="
                    default-src 'self';
                    script-src 'nonce-${editorScriptKey}';
                    style-src 'unsafe-inline' ${panel.webview.cspSource};
                    img-src 'self' data: ${panel.webview.cspSource};
                    font-src 'self' data: ${panel.webview.cspSource};
                    frame-src 'self' ${trustEmbedDomains.join(' ')}; ${panel.webview.cspSource}
                ">
                <link href="${panel.webview.asWebviewUri(stylesheet)}" rel="stylesheet">
                <style>
                    #overlay {
                        display: none;
                        position: fixed;
                        top: 0;
                        left: 0;
                        right: 0;
                        bottom: 0;
                        padding: 1rem;
                        opacity: 0.8;
                        z-index: 99999;
                    }
                    #root.container {
                        min-width: 100vw;
                        min-height: 100vh;
                    }
                    #preview {
                        display: none;
                        position: absolute;
                        top: 0;
                        left: 0;
                        right: 0;
                        bottom: 0;
                        display: flex;
                        justify-items: center;
                        justify-content: center;
                        align-items: center;
                        width: 100vw;
                        height: 100vh;
                        align: center;
                        vertical-align: center;
                    }
                    #preview > svg {
                        max-width: 100vw;
                        max-height: 100vh;
                    }
                </style>
                <script src="${panel.webview.asWebviewUri(editorScript)}" nonce="${editorScriptKey}" type="module"></script>
            </head>
            <body>
                <div id="overlay">
                    Loading ..
                </div>
                <main id="root" class="container">
                    <!--TODO-->
                <main>
                <aside id="preview">
                    ${doc.getText()}                    
                </aside>
            </body>
            </html>
        `
    }
}

