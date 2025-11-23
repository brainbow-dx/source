// deno-lint-ignore-file
import { join, resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

import { build } from "@deno/dnt";

// import esbuild from "npm:esbuild@latest";

//---
const args = sh.parse<Args>(Deno.args);
const production = Deno.args.includes("--production");
const watch = Deno.args.includes("--watch");

args.workdir ??= resolve(import.meta.dirname!, "..");

//---
// import packageManifest from "../assets/vscode/package.json" with { type: "json" };

await build({
	entryPoints: [
		join(args.workdir, "./src/extension.ts"),
	],
	outDir: join(args.workdir, "./.output/pkg/vscode"),
	shims: {
		// see JS docs for overview and more options
		deno: true,
	},
	package: {
		name: "sketch-vscode",
		version: "0.0.0",
		description: "TODO",
		license: "MIT",
		repository: {
			type: "git",
			url: "git+https://github.com/brainbow-dev/sketch-vscode.git",
		},
		bugs: {
			url: "https://github.com/brainbow-dev/sketch-vscode/issues",
		},
		engines: {
			vscode: "^1.74.0"
		},
		activationEvents: [
			"onStartupFinished",
		],
	},
	async postBuild() {
		Deno.chdir(join(args.workdir, "./.output/pkg/vscode"));

		// TODO: Add licenses and readm + assets ..

		await $`npx vsce package`;

		// TODO: Probably bundle the front-end code here?
		// await $`deno bundle ${join(args.workdir, "./src/mod.ts")} \\
		//     --outdir ${join(args.workdir, "./.output/pkg/deno")} \\
		//     --allow-import --allow-scripts  \\
		//     --code-splitting --sourcemap  \\
		//     --platform browser`;
		// await $`deno bundle ${join(args.workdir, "./assets/vscode/index.html")} \\
		//     --outdir ${join(args.workdir, "./.output/pkg/web/public")} \\
		//     --allow-import --allow-scripts  \\
		//     --code-splitting --sourcemap  \\
		//     --platform browser`;
	},
});


// const ctx = await esbuild.context({
// 	entryPoints: [
// 		"extension.ts"
// 	],
// 	bundle: true,
// 	format: "cjs",
// 	minify: production,
// 	sourcemap: !production,
// 	sourcesContent: false,
// 	platform: "node",
// 	outfile: "dist/extension.js",
// 	external: ["vscode"],
// 	logLevel: "silent",
// 	plugins: [
// 		{
// 			name: "esbuild-problem-matcher",
// 			setup(build: any) {
// 				build.onStart(() => {
// 					console.log("[watch] build started");
// 				});
// 				build.onEnd((result: any) => {
// 					result.errors.forEach(({ text, location }: any) => {
// 						console.error(`✘ [ERROR] ${text}`);
// 						console.error(`    ${location.file}:${location.line}:${location.column}:`);
// 					});
// 					console.log(`[watch] build finished`);
// 				});
// 			},
// 		},
// 	],
// });

// if (watch) {
// 	await ctx.watch();
// } else {
// 	await ctx.rebuild();
// 	await ctx.dispose();
// }
