import type { JSX } from "react";

export type DrawFn = (props: object) => JSX.Element | Promise<JSX.Element>;
