import denoConfigText from "../deno.jsonc" with { type: "text" };
const denoConfig = JSON.parse(denoConfigText);

export const VERSION = denoConfig.version;

// @struct("Dang")
export class Dang {
    // @asdf()
    something: string = "";
}

function init(woop: string) {
    //..
}
