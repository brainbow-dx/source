// A slash-command script — see `runtimes/terminal/examples/assistant.rs`'s "JS commands"
// section. Every `.js` file in this directory becomes a `/<filename>` command; its exported
// `run(args)` is called with whatever text follows the command name, and its return value (plus
// anything it `console.log`s along the way) becomes the assistant's reply.
//
// This one demonstrates progress reporting: each `console.log` call streams into the transcript
// live, as it happens, rather than waiting for the whole script to finish (see `TranscriptLayer`
// and `run_js_command` in `assistant.rs`). No special API for this, `console.log` is enough.

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function run() {
    const steps = ["Starting up", "Working on step 1", "Working on step 2", "Almost done"];

    for (const step of steps) {
        console.log(step);
        await sleep(600);
    }

    return "Done.";
}
