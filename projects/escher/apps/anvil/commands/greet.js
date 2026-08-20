// A slash-command script — see `runtimes/terminal/examples/assistant.rs`'s "JS commands"
// section. Every `.js` file in this directory becomes a `/<filename>` command; its exported
// `run(args)` is called with whatever text follows the command name, and its return value (plus
// anything it `console.log`s along the way) becomes the assistant's reply.

export function run(args) {
    const name = args.trim() || "world";
    return `Hello, ${name}! (this reply came from a real JS script, run via ethos-deno)`;
}
