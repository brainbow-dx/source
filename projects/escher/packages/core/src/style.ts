// A serializable mirror of `escher_core::style`'s `Property` variants — the JSON shape
// `runtimes/web/src/description.rs`'s `StyleDescription`/`ValueDescription`/`EdgeDescription`
// deserialize (field names and `#[serde(rename_all = "camelCase")]` tag values match exactly).
// Only the subset `surface.rs` currently maps to CSS is covered — extend both sides together.

export type Value =
    | { unit: "auto" }
    | { unit: "px"; value: number }
    | { unit: "percent"; value: number }
    | { unit: "fill"; value: number };

export const px = (value: number): Value => ({ unit: "px", value });
export const percent = (value: number): Value => ({ unit: "percent", value });
export const fill = (value: number): Value => ({ unit: "fill", value });
export const auto: Value = { unit: "auto" };

export type Edge = "all" | "top" | "right" | "bottom" | "left";

export type FlexDirectionValue = "row" | "column";

export type Style =
    | { type: "size"; width?: Value; height?: Value }
    | { type: "margin"; edge: Edge; value: Value }
    | { type: "padding"; edge: Edge; value: Value }
    | { type: "gap"; value: Value }
    | { type: "flex"; grow: number }
    | { type: "flexDirection"; direction: FlexDirectionValue }
    | { type: "backgroundColor"; color: string }
    | { type: "contentColor"; color: string };

export const Padding = {
    all: (value: Value): Style => ({ type: "padding", edge: "all", value }),
    top: (value: Value): Style => ({ type: "padding", edge: "top", value }),
    right: (value: Value): Style => ({ type: "padding", edge: "right", value }),
    bottom: (value: Value): Style => ({ type: "padding", edge: "bottom", value }),
    left: (value: Value): Style => ({ type: "padding", edge: "left", value }),
};

export const Margin = {
    all: (value: Value): Style => ({ type: "margin", edge: "all", value }),
    top: (value: Value): Style => ({ type: "margin", edge: "top", value }),
    right: (value: Value): Style => ({ type: "margin", edge: "right", value }),
    bottom: (value: Value): Style => ({ type: "margin", edge: "bottom", value }),
    left: (value: Value): Style => ({ type: "margin", edge: "left", value }),
};

export const Size = {
    width: (value: Value): Style => ({ type: "size", width: value }),
    height: (value: Value): Style => ({ type: "size", height: value }),
    xy: (value: Value): Style => ({ type: "size", width: value, height: value }),
};

export const Gap = (value: Value): Style => ({ type: "gap", value });

export const Flex = (grow: number): Style => ({ type: "flex", grow });

export const FlexDirection = {
    row: { type: "flexDirection", direction: "row" } as Style,
    column: { type: "flexDirection", direction: "column" } as Style,
};

export const BackgroundColor = (color: string): Style => ({ type: "backgroundColor", color });

export const ContentColor = (color: string): Style => ({ type: "contentColor", color });
