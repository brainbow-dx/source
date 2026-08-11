import { commands, window } from "vscode";
import type { ExtensionContext } from "vscode";

import { TracingSubscriber } from "./tracing.ts";

import { SketchExplorer } from "./view/explorer.tsx";
import { SketchInspector } from "./view/inspector.tsx";
import { SketchPad } from "./view/pad.tsx";

import { SketchEditor } from "./editor/sketch.tsx";
import { MarkupEditor } from "./editor/markup.tsx";
import { MarkdownEditor } from "./editor/markdown.tsx";

// Note: Use Node's child process manager to run commands:
// import { exec } from "node:child_process";
// import type { ChildProcess } from "node:child_process";

// let langServiceProc: ChildProcess | undefined = undefined;

//---
export function activate(context: ExtensionContext) {
    const tracer = new TracingSubscriber(window.createOutputChannel("Escher"));
    context.subscriptions.push(tracer.channel);

    // 
    tracer.info(`Escher VSCode Ext starting ..`);

    //--
    // TODO: Start and/or connect to the configured Dev Service.
    // langServiceProc = exec(`[some-command]`, { shell: "bash" });
    // // deno-lint-ignore no-constant-condition
    // if (false && langServiceProc) {
    //     tracer.debug(`Lang Service:`, JSON.stringify(langServiceProc, null, 2));
    // }

    tracer.debug(`Status: Ok`);
    context.subscriptions.push((
        commands.registerCommand("escher-vscode.helloWorld", () => {
            window.showInformationMessage(`Hello World from Escher!`);
            tracer.debug(`Hello World from Escher! <3`);
        })
    ));

    //--
    const sketchExplorer = new SketchInspector(context.extensionUri, tracer);
    context.subscriptions.push((
        window.registerWebviewViewProvider(`escher-explorer`, sketchExplorer)
    ));

    //--
    const sketchInspector = new SketchInspector(context.extensionUri, tracer);
    context.subscriptions.push((
        window.registerWebviewViewProvider(`escher-inspector`, sketchInspector, {
            webviewOptions: {
                retainContextWhenHidden: true,
            },
        })
    ));

    //--
    const sketchPad = new SketchPad(context.extensionUri, tracer);
    context.subscriptions.push((
        window.registerWebviewViewProvider(`escher-pad`, sketchPad, {
            webviewOptions: {
                retainContextWhenHidden: true,
            },
        })
    ));

    // //--
    const sketchEditor = new SketchEditor(context.extensionUri, tracer);
    context.subscriptions.push((
        window.registerCustomEditorProvider(`escher-editor`, sketchEditor, {
            webviewOptions: {
                retainContextWhenHidden: true
            }
        })
    ));

    // // //--
    const markupEditor = new MarkupEditor(context.extensionUri, tracer);
    context.subscriptions.push((
        window.registerCustomEditorProvider(`escher-markup-editor`, markupEditor, {
            webviewOptions: {
                retainContextWhenHidden: true
            }
        })
    ));

    // //--
    const markdownEditor = new MarkdownEditor(context.extensionUri, tracer);
    context.subscriptions.push((
        window.registerCustomEditorProvider(`escher-markdown-editor`, markdownEditor, {
            webviewOptions: {
                retainContextWhenHidden: true
            }
        })
    ));
}

export function deactivate() {
    // TODO: Gracefully, pls ..
    // langServiceProc?.kill();
}
