// deno-lint-ignore-file no-explicit-any
import { debounce } from "@std/async";
// import { generate } from "@std/uuid/v5";

import type { Workspace } from "@ethos/core/workspace";
import type { ManagedProcess } from "#src/shell.ts";

//---
export enum DevServerMessageKind {
    unknown = 0,
}

export interface DevServerMessage {
    kind?: DevServerMessageKind,
    from?: string, // UUID
    to?: string[], // UUID[]
    body?: string,
}

export interface DevServerOptions {
    hostname?: string;
    port?: number;
    serveDir?: string;
    outputDir?: string;
}

export class DevServer {
    #server?: Deno.HttpServer;
    
    constructor(
        public readonly workspace: Workspace,
        private broadcast?: BroadcastChannel,
        public readonly options: DevServerOptions = {
            hostname: "localhost",
            port: 9000,
            outputDir: "./.output",
        },
    ) {
        // TODO: this.#server = new Server();
        
        const channelKey = `TODO`;
        this.broadcast ??= new BroadcastChannel(channelKey);
    }
    
    public sendBroadcast(message: DevServerMessage) {
        this.broadcast?.postMessage(message);
    }
    
    public start() {
        // TODO: Use the Atlas Server (this.#server) instance here ..
        this.#server = Deno.serve(this.options, this.handleRequest.bind(this));
    }
    
    public stop() {
        this.#server?.shutdown();
    }
    
    private handleRequest(request: Request) {
        const url = new URL(request.url);
        
        if (request.headers.get("upgrade") !== "websocket") {
                const { socket, response } = Deno.upgradeWebSocket(request);
                
                socket.addEventListener("open", () => {
                    console.log("a client connected!");
                });
                
                socket.addEventListener("message", (event) => {
                    if (event.data === "ping") {
                        socket.send("pong");
                    }
                });
                
                return response;
        }
        
        return new Response(`Hola, worl'! You are requesting a "${request.destination}" ..`);
    }
}

export interface ServeDevOptions extends DevServerOptions {
    watchDirs?: string[],
    startServer?: boolean | (() => boolean),
    runOnStartup?: boolean | (() => boolean),
    onFsEvent?(event: Deno.FsEvent): void,
}

export async function serveDev(workspace: Workspace, process?: ManagedProcess, options?: ServeDevOptions): Promise<any> {
    if (workspace.root == undefined) {
        return console.warn(`Workspace has no root.`, workspace);
    }
    
    const devServer = new DevServer(workspace, undefined, {
        outputDir: options?.outputDir ?? "./.output",
        hostname: options?.hostname ?? "localhost",
        port: options?.port ?? 9000,
    });
    
    if (options?.startServer !== false) {
        devServer.start();
        process?.run();
    }
    
    if (options?.onFsEvent) {
        const onFsEvent = debounce(options.onFsEvent, 500);
        
        if (options?.runOnStartup === true) {
            onFsEvent({ kind: "other", paths: [] });
        }
        
        console.debug(`Watching for changes in '${workspace.root}' ..`);
        
        const watchDirs: Array<string> = [
            // TODO: Dependency dirs (from workspace inspection) ..
            workspace.root.toString(),
        ];

        for await (const event of Deno.watchFs(watchDirs, { recursive: true })) {
            // TODO: Send filtered changes to the front-end, SIR.
            
            if (event.kind == "access") {
                continue;
            }
            
            let shouldUpdate = false;
            let foundChanges = 0;
            
            for (const eventPath of event.paths) {
                if (options.outputDir && eventPath.startsWith(options.outputDir)) {
                    continue;
                }
                
                shouldUpdate = true;
                foundChanges++;
                
                devServer.sendBroadcast({
                    kind: DevServerMessageKind.unknown,
                    
                });
            }
            
            if (shouldUpdate && options.onFsEvent instanceof Function) {
                onFsEvent(event);
            }
        }
    }
}
