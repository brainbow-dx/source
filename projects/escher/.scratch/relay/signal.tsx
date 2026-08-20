
function handler(request: Request): Response {
    const { pathname } = new URL(request.url);

    switch (pathname) {
    case "/ws":
        return handleWebSocket(request);
    case "/":
        return new Response("MCP Server is running!", { status: 200 });
    default:
        return new Response("Not found", { status: 404 });
    }
}

function handleWebSocket(request: Request): Response {
    const { socket, response } = Deno.upgradeWebSocket(request);
    
    switch (request.method) {
    case "GET":
    default:
        handleSocket(socket);
        return response;
    }
}

function handleSocket(sock: WebSocket) {
  console.log("MCP client connected.");

  sock.onopen = () => {
    console.log("WebSocket connection opened.");
    // Optionally send a welcome message or initial data
    // sock.send(JSON.stringify({ message: "Welcome to MCP" }));
  };

  sock.onmessage = (e) => {
    console.log("Received message:", e.data);
    // Echo the message back to the client
    if (sock.readyState === WebSocket.OPEN) {
      sock.send(`Echo: ${e.data}`);
    }
  };

  sock.onerror = (e) => {
    console.error("WebSocket error:", e);
  };

  sock.onclose = () => {
    console.log("MCP client disconnected.");
  };
}

// Start the HTTP server
console.log("Starting MCP server on http://localhost:8080");
Deno.serve({ port: 8080 }, handler);