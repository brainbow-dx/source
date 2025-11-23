import denoConfig from "../deno.json" with { type: "json" };

export const VERSION = denoConfig.version;

// @struct("Dang")
export class Dang {
    // @asdf()
    something: string = "";
}

function init(woop: string) {
    //..
}
