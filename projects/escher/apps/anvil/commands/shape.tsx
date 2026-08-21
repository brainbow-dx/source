/** @jsxImportSource @escher/jsx */
// The `/shape` demo's actual source of truth is authored here, in Anvil (the project that "houses"
// the command), as JSX. Shape-authoring belongs where the command lives, not in Ethos.
// `@escher/jsx` (see `packages/jsx`) compiles this into a `ScaffoldNode` tree matching
// `runtimes/web/src/description.rs`'s `ScaffoldDescription` schema exactly. It's the same shape
// `packages/jsx/examples/render.ts` produces.
//
// Run directly: `deno run commands/shape.tsx` (needs `apps/anvil` in the Deno workspace; see
// the root `deno.jsonc`, so `@escher/jsx`/`@escher/core` resolve). Prints the `ScaffoldNode` as
// JSON to stdout when run as the main module. `apps/anvil/src/process.rs`'s `run_deno_command`
// invokes exactly this.
//
// What happens to the output from here: `apps/anvil/src/shape.rs` takes this JSON and hands it,
// unmodified, to `ethos/tools/codegen/uxml/from-description.ts` (`ethos-cli run-command`). That is
// a pure `ScaffoldDescription → {uxml, uss}` transform, no authored content of its own. The only
// thing between "built here in Escher" and "compiled in Ethos" is a plain JSON handoff between
// two process invocations, nothing more.

import { BackgroundColor, ContentColor, FlexDirection, Gap, Padding, px } from "@escher/core/style";
import type { Style } from "@escher/core/style";

const COLOR = "#7aa2f7";
const WIDTH_PX = 240;
const HEIGHT_PX = 140;

const BOX_SIZE: Style = { type: "size", width: px(WIDTH_PX), height: px(HEIGHT_PX) };

export function Shape() {
    return (
        <box style={[FlexDirection.column, Gap(px(16)), Padding.all(px(24))]}>
            <box style={[BackgroundColor(COLOR), BOX_SIZE]} />
            <box style={[ContentColor("#e0e0e0"), { type: "size", width: px(420), height: { unit: "auto" } }]}>
                {`Escher demo shape: a ${WIDTH_PX}×${HEIGHT_PX}px box filled ${COLOR}. `}
                {`This page is static HTML rendered by escher-web's SSG path (Anvil's /shape command, `}
                {`via escher_web::ssg::render_page_to_html) -- not a live game canvas or Unity view. `}
                {`The same description is also compiled to Unity UXML/USS `}
                {`(Assets/UI/Generated/Shape.{uxml,uss} in Aby's Unity project) but nothing loads or `}
                {`displays it in a running Unity scene yet -- that file just sits on disk for a human to `}
                {`wire up manually.`}
            </box>
        </box>
    );
}

if (import.meta.main) {
    // Plain `console.log` throughout. It already streams live into Anvil's output UI the same
    // way `ethos-cli run-command`'s own `console.log` passthrough does (see
    // `apps/anvil/src/process.rs`'s `run_streamed_command`), so there's no separate "logging
    // utility" to think about on the scripting side. The caller (`apps/anvil/src/shape.rs`) reads
    // only the *last* line of stdout as the real payload, matching `ethos-cli run-command`'s own
    // "console.log output, then the return value, last" convention. So any number of progress
    // lines before it are free, so `/shape` shows what it's doing while it runs instead of going
    // silent until it's done.
    console.log("Building scaffold from JSX...");
    const scaffold = Shape();
    console.log(`Scaffold built (${scaffold.children.length} top-level children).`);
    console.log(JSON.stringify(scaffold));
}
