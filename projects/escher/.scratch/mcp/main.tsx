// deno-lint-ignore-file
import { join, resolve } from "jsr:@std/path";
import { exists } from "jsr:@std/fs";
import { serveDir, serveFile } from "jsr:@std/http";

import { JrpcEventStream } from "./jrpc.tsx";

//---
Deno.serve({ port: 9090 }, handle);

//---
const rootDir = resolve(import.meta.dirname!, "../..");
const publicDir = join(rootDir, `./.output/pkg/web/public`);

console.info(`Root Dir:`, rootDir);
console.info(`Public Dir:`, publicDir);

const signalConnections = new Map<number, any>();

const mcpConnections = new Map<number, JrpcEventStream>();

async function handle(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const pathname = url.pathname;
    
    if (pathname.startsWith("/mcp")) {
        const acceptHeader = request.headers.get("accept")?.toLowerCase();
        if (acceptHeader?.toLowerCase().includes("text/event-stream")) {
            console.log(`EventStream Request:`, request);
            return await streamContextResponse(request);
        }
    }
    
    if (pathname.startsWith("/coop")) {
        const connectionHeader = request.headers.get("connection");
        if (connectionHeader?.toLowerCase().includes("upgrade")) {
            console.log(`WebSocket Request:`, request);
            return await startSignalingSession(request);
        }
    }
    
    if (pathname.endsWith("com.chrome.devtools.json")) {
        const devtoolsConfig = {
            workspace: {
                root: "/Users/lorren/Dev/Agents/apps/terminal/.output/pkg/web/public",
                uuid: 'my-uuid-xxx',
            },
        };
        return new Response(JSON.stringify(devtoolsConfig, null, 2));
    }
    
    if (pathname.startsWith("/admin")) {
        console.log(`Admin Request:`, request);
        // TODO: Deny if user isn't elevated ..
    }
    
    if (pathname.startsWith("/preview")) {
        console.log(`Preview:`, request);
        return new Response("Preview", { status: 200 });
    }
    
    if (await exists(join(publicDir, pathname))) {
        // The built-in serveDir method fails to load files with weird/complex
        // patterns, such as those output by deno's bundling tools.
        let servePath = resolve(join(publicDir, pathname));
        if (pathname.match(/\/$/i)) {
            servePath = join(servePath, "index.html");
            if (!(await exists(servePath))) {
                return new Response('Not Found', { status: 200 });
            }
        }
        return serveFile(request, servePath)
    }
    
    console.log(`Unhandled Request:`, request);
    return new Response("Not Found", { status: 404 });
}

async function startSignalingSession(request: Request): Promise<Response> {
    const sessionIDx = signalConnections.keys.length;
    const { socket, response } = Deno.upgradeWebSocket(request);
    
    console.debug("MCP client connected.");
    
    socket.onopen = event => {
        console.log("Connected:", event);
        // Optionally send a welcome message or initial data
        // sock.send(JSON.stringify({ message: "Welcome to MCP" }));
    };
    
    socket.onmessage = message => {
        console.debug("Received message:", message.data);
        // Echo the message back to the client
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(`Echo: ${message.data}`);
        }
    };
    
    socket.onerror = error => {
        console.error("WebSocket error:", error);
    };
    
    socket.onclose = event => {
        console.debug(`Disconnect:`, event);
    };
    
    signalConnections.set(sessionIDx, socket);
    
    return response;
}

async function streamContextResponse(request: Request): Promise<Response> {
    // TODO: Get this from a generic factory using request token..
    const clientIDx = mcpConnections.entries.length;
    const connection = new JrpcEventStream({
        id: clientIDx,
    });
    
    // TODO: ..
    mcpConnections.set(clientIDx, connection)
    
    return new Response(new ReadableStream(connection), {
        status: 200,
        headers: {
            // TODO: Get CORS from env/configs.
            "Access-Control-Allow-Origin": "*",
            "Content-Type": "text/event-stream",
        },
    });
}
