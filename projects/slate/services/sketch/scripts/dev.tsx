#!/usr/bin/env deno
// deno-lint-ignore-file

import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

import { ReactNode, useState } from "react";

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

export interface TerminalSurfaceProps {
    //..
}

//---
const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";

Deno.chdir(args.workdir);

const terminal = new TerminalSurface(Deno.stdout);

// TODO: Replace this with a banner UI panel?
console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Flutter Exe:`, await sh.which("flutter"));

await $`deno run -A --unstable-bundle ./scripts/build.tsx`;

// Note: We don't get to this point. Everything below is 
// invisible to 
terminal.draw(async ({ }: object) => {
    const devService = new DevService();
    await $`cargo run -p slate-sketch --example serve -- --address 0.0.0.0:3000`;

    return (
        <DashboardApp>
            <header>
                Goodbye!
            </header>
            <section>
                <aside>
                    TODO
                </aside>
                <article>
                    TODO
                </article>
            </section>
            <footer>
                <section>
                    <form>
                        <section>
                            <fieldset>
                                <label>TODO</label>
                                <input type="text" />
                            </fieldset>
                        </section>
                    </form>
                </section>
            </footer>
        </DashboardApp>
    )
});

//---
export interface DashboardAppProps {
    router?: any; // TODO: What router should we use?
    children?: ReactNode;
}

export function DashboardApp(props: DashboardAppProps) {
    return (
        <DashboardRouter $ref={props.router}>
            <header>
                Some Page?
            </header>
        </DashboardRouter>
    )
}

export interface DashboardRouterProps {
    $ref?: object, // TODO
    children?: React.ReactNode;
}

export function DashboardRouter(props: DashboardRouterProps) {
    return repeat(1, async () => (
        <div className={["container"].join(',')}>
            {props.children}
        </div>
    ))
}
