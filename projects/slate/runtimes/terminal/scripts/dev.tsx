#!/usr/bin/env deno
// deno-lint-ignore-file

import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

import { TerminalSurface } from "@brainbow/slate/terminal";

//---
export function repeat(_n: number, _callback: () => any) {
    return undefined;
}

export class DevService {
    constructor() {
        //..
    }
}

interface TerminalSurfaceProps {
    //..
}

//---
const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";

Deno.chdir(args.workdir);

const terminalSurface = new TerminalSurface(Deno.stdout);

// TODO: Replace this with a banner UI panel?
console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Flutter Exe:`, await sh.which("flutter"));

await $`deno run -A ./scripts/build.tsx`;

terminalSurface.draw(async (_props: TerminalSurfaceProps) => {
    const devService = new DevService();
    const devProc = $`cargo run -p slate-terminal --example serve`;
    const devState = new Map<number, Set<string>>();

    return (
        <Dashboard $state={devState}>
            <header>
                Hello.
            </header>
        </Dashboard>
    )
});

interface DashboardProps {
    $state: any;
    children?: React.ReactNode;
}

export function Dashboard(_props: DashboardProps) {
    const dashboardRouter = undefined;

    // deno-lint-ignore require-await
    return (
        <DashboardRouter $ref={dashboardRouter}>
            <header>
                Some Page?
            </header>
        </DashboardRouter>
    )
}

interface DashboardRouterProps {
    $ref?: object, // TODO
    children?: React.ReactNode;
}

export function DashboardRouter(props: DashboardRouterProps) {
    return repeat(1, async () => (
        <div className="container">
            {props.children}
        </div>
    ))
}
