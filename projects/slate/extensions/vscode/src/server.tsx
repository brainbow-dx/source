import { join } from "@std/path";

import { renderToString } from "react-dom/server";

function Application() {
    return (
        <div>Hello??</div>
    )
};

export interface StartOptions {
    port?: number;
}

export function start(options?: StartOptions) {
    // Start the Deno server on port 3000.
    const port = options.port ?? 3000;

    console.log(`Server is running at http://localhost:${port}`);

    Deno.serve({
        port,
        // TODO: onServe (or whatever).
    }, (request) => {
        const url = new URL(request.url);

        if (url.pathname === "/") {
            return new Response(renderToString(<Application />), {
                headers: { "Content-Type": "text/html" },
            });
        }

        if (url.pathname) {
            try {
                const workdir = join(Deno.cwd(), ".");
                const file = await Deno.readTextFile(join(workdir, url.pathname));

                return new Response(file, {
                    headers: {
                        "Content-Type": "text/javascript",
                    },
                });
            } catch (e) {
                console.error("Error reading file:", e);
                return new Response("Not Found", { status: 404 });
            }
        }

        // Handle all other requests with a 404 Not Found response.
        return new Response("Not Found", { status: 404 });
    });
}
