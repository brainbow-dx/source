import { BackgroundColor, ContentColor, FlexDirection, Gap, Padding, px } from "@escher/core/style";

export function Page() {
    return (
        <box style={[Padding.all(px(24)), Gap(px(12)), FlexDirection.column, BackgroundColor("#1a1a1a")]}>
            <box style={ContentColor("#f5f5f5")}>Hello from JSX</box>
            <box style={ContentColor("#a0a0a0")}>This scaffold was authored in TSX and compiled through @escher/jsx.</box>
        </box>
    );
}
