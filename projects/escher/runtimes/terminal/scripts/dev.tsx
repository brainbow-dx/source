#!/usr/bin/env deno
// deno-lint-ignore-file
import { resolve } from "@std/path";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

import { TerminalSurface } from "@escher/terminal";

//---
const args = sh.parse<sh.Args>(Deno.args);
console.log(`Args:`, JSON.stringify(args, null, 2));

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.example ??= "ratatui";

Deno.chdir(args.workdir);

const terminal = new TerminalSurface(Deno.stdout);

// TODO: Replace this with a banner UI panel?
console.info(`Work Dir: ${Deno.cwd()}`);

await $`deno run -A ./scripts/build.tsx`;

terminal.draw(async props => {
    // TODO: Attach to a dev server to the running script for use in updating the ui.
    // const devService = useCommand($`cargo run -p escher-terminal --example ${args.example}`);
    await $`cargo run -p escher-terminal --example ${args.example}`;
    
    return (
        <DevServiceReport data={new Map<number, Set<string>>()}>
            <header title="Command Result">
                TODO: Report on status of the above!
            </header>
        </DevServiceReport>
    )
});

//---
// TODO: Move all of this (below) to the DevService implementation.
export function repeat(_n: number, _callback: () => any)
{
    return undefined;
}

export class DevService {
    constructor(proc: any) {
        //..
    }
}

interface DevServiceReportProps {
    data: Map<number, any> | undefined;
    children?: React.ReactNode;
}

export function DevServiceReport(_props: DevServiceReportProps) {
    const dashboardRouter = undefined;

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
