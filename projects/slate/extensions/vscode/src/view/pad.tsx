import * as vscode from "vscode";
import { Uri } from "vscode";
import type { CancellationToken } from "vscode";
import type { TextDocument, CustomTextEditorProvider } from "vscode";
import type { Webview, WebviewPanel, WebviewView, WebviewViewProvider, WebviewViewResolveContext } from "vscode";

import { parseMarkup } from "@brainbow/slate/parse/markup";

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

//---
export class SketchPad implements WebviewViewProvider {
    private catalog: Array<string> = [];
    private view?: WebviewView;

    constructor(
        private readonly extensionUri: Uri,
        private readonly tracer: TracingSubscriber,
    ) {
        //..
    }

    public resolveWebviewView(
        view: vscode.WebviewView,
        context: vscode.WebviewViewResolveContext,
        token: vscode.CancellationToken,
    ) {
        const sketchFiles = searchWorkspaceFiles("**/**.sketch.svg");
        this.tracer.debug(`SVG Files:`, JSON.stringify(sketchFiles, null, 2));

        view.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                this.extensionUri,
            ]
        };

        view.webview.onDidReceiveMessage((message: any) => {
            switch (message.command) {
                case 'request:setSelectedSketch':
                    // Handle the message, e.g., apply a workspace edit to the document
                    this.tracer.debug('Received update:', message.text);
                    return;
            }
        });

        view.webview.html = this.draw(view);

        this.view = view;
    }

    public draw(view: WebviewView): string {
        const dbgPanel = JSON.stringify(view, null, 2);
        this.tracer.debug(`Drawing Inspector:`, dbgPanel);

        const assetDir = Uri.joinPath(this.extensionUri, "web/assets");

        const stylesheet = Uri.joinPath(assetDir, "editor/sketch.css");

        const editorScript = Uri.joinPath(assetDir, "editor/sketch.js");
        const editorScriptKey = generateScriptKey(editorScript);

        // TODO: Move to config/registry and/or settings.
        const trustEmbedDomains = [
            "vscode-webview:",
            "http://localhost",
            "http://127.0.0.1",
            "http://*.stooper.local",
            "https://*.brainbow.local",
            "https://www.youtube.com",
            "https://cdn.jsdelivr.net/npm/@excalidraw/excalidraw",
        ];

        return `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <title>Sketch Inspector</title>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta http-equiv="Content-Security-Policy" content="
                    default-src 'self';
                    script-src 'nonce-${editorScriptKey}';
                    style-src 'unsafe-inline' ${view.webview.cspSource};
                    img-src 'self' data: ${view.webview.cspSource};
                    font-src 'self' data: ${view.webview.cspSource};
                    frame-src 'self' ${trustEmbedDomains.join(' ')}; ${view.webview.cspSource}
                ">
                <link href="${view.webview.asWebviewUri(stylesheet)}" rel="stylesheet">
                <script src="${view.webview.asWebviewUri(editorScript)}" nonce="${editorScriptKey}" type="module"></script>
            </head>
            <body>
                <div id="root" class="container">
                    Loading ??
                </div>
            </body>
            </html>
        `
    }
}
