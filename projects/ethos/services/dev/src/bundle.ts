
export interface BundleOptions<B extends Bundler> {
    bundler?: B,
    watch?: boolean,
}

export async function bundle(entrypoints: string | string[], options: BundleOptions<DenoBundler>) {
    options.bundler ??= new DenoBundler();

    if (!Array.isArray(entrypoints)) {
        entrypoints = [entrypoints];
    }

    //--
    const { bundler } = options;

    return await bundler.run(entrypoints);
}

//--
export interface Bundler {
    run(): Promise<DenoBundleResult>;
}

export type DenoBundleResult = Deno.bundle.Result;

export class DenoBundler implements Bundler {
    constructor(
        //..
    ) {
        //..
    }

    public async run(entrypoints: string[] = []) {
        const browserBundle = await Deno.bundle({
            entrypoints: entrypoints,
            outputDir: ".output/pkg/assets/web",
            codeSplitting: true,
            inlineImports: true,
            // sourcemap: "inline",
            platform: "browser",
            // packages: "bundle",
            format: "esm",
            minify: false,
            write: false,
        });

        console.debug(`Browser Bundle:\n`, browserBundle);
        return browserBundle;
    }
}
