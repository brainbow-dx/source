#!/usr/bin/env deno
import { globToRegExp, resolve, parse } from "@std/path";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";
import { ManagedProcess } from "@ethos/dev/shell";

import { serveDev } from "@ethos/dev/server";

import { Workspace } from "@ethos/core/workspace";

// import { CaddyClient, buildServiceRoutes } from "caddy";

//---
export interface Args extends sh.Args {
    //..
}

const args = sh.parse<Args>(Deno.args, args => {
    args.cwd ??= resolve(import.meta.dirname!, "..");
    args.hostname ??= Deno.env.get("DEV_SERVER_HOSTNAME") ?? "0.0.0.0";
    args.port ??= Deno.env.get("DEV_SERVER_PORT") ?? (9000).toString();
    args.server ??= true;
    args.generate ??= true;
    args.build ??= false;
    args.run ??= true;
    args.clean ??= false;

    Deno.chdir(args.cwd);
});

if (args.build) {
    await $`deno task build --reset --generate`;
}

const composeProcess = new ManagedProcess($`
    docker compose --project-name escher-dev -f ./compose.yaml --profile web --profile data up --build -d
`, {
    // delay: 1000,
    timeout: 10000,
});

if (args.server) {
    await composeProcess.run();
}

if (args.run) {
    // const caddyClient = new CaddyClient({
    //     adminUrl: "http://127.0.0.1:2019",
    // });
    // 
    // const serverName = "srv0"; // TODO: Get from cli/env?
    // await caddyClient.patchServer({
    //     [serverName]: {
    //         listen: ["443"],
    //         routes: buildServiceRoutes({
    //             host: "dev-room-001.escher.localhost",
    //             dial: "127.0.0.1:3615",
    //             securityHeaders: {
    //                 enableHsts: true,
    //                 frameOptions: "DENY",
    //             },
    //         }),
    //     },
    // });
    // const servers = await caddyClient.getServers();
    // console.debug(`Servers:`, JSON.stringify(servers, null, 2));
    // 
    // await caddyClient.addRoutes("https_server", buildServiceRoutes({
    //     host: "dev-room-001.escher.localhost",
    //     dial: "127.0.0.1:3615",
    //     securityHeaders: {
    //         enableHsts: true,
    //         frameOptions: "DENY",
    //     },
    // }));
    
    const devProcess = new ManagedProcess($`
        cargo run -p escher --example terminal --features dev -- \
            --cwd ${args.cwd} \
            --address 127.0.0.1:3615
    `, {
        delay: 100,
        timeout: 5000,
    });
    
    const workspace = new Workspace(args.cwd);
    const outputDirRx = globToRegExp("**/.output/**");
    
    await serveDev(workspace, devProcess, {
        hostname: args.hostname,
        port: parseInt(args.port),
        outputDir: args.outdir,
        async onFsEvent(event: any) {
            try {
                let shouldBuild = false;
                let shouldRestart = false;
                
                for (const eventPath of event.paths) {
                    const path = parse(eventPath);
                    
                    if (outputDirRx.test(path.dir)) {
                        continue; // Skip artifacts ..
                    }
                    else if (/\.(html|htmx|md|mdx|svg|css|json?)$/.test(path.ext)) {
                        shouldBuild = true;
                    }
                    else if (/\.(rs|ts|tsx?)$/.test(path.ext)) {
                        shouldBuild = true;
                        shouldRestart = true;
                    }
                    else if (["Cargo.toml", "deno.json"].includes(path.base)) {
                        shouldBuild = true;
                        shouldRestart = true;
                    }
                }
                
                if (devProcess.isRunning() && shouldRestart) {
                    // TODO: Try to run shutdown operations first?
                    await devProcess.kill();
                }
                
                if (args.build && shouldBuild) {
                    await $`deno task build`;
                }
                
                if (args.server && !composeProcess.isRunning() && shouldRestart) {
                    // Starts, but does not restart compose services.
                    // Note: Compose services sometimes manage their own watch
                    //  mechanism, and restarting could interfere.
                    await composeProcess.run();
                }
                
                if (args.run && !devProcess.isRunning() && shouldRestart) {
                    await devProcess.run();
                }
            } catch (error: unknown) {
                // TODO: Optionally alert the user?
                console.error(`Dev build failed!`, error);
                alert(`Dev build failed: ${error}`);
            }
        },
    })
}

if (args.server && args.shutdown) {
    await $`docker compose -f ./compose.yaml down`;
}

if (args.clean) {
    await $`deno task clean`;
}
