import type { JSX } from "react";

/**
 * Represents a function that can draw on a surface, and produces 
 */
export type DrawFn<T> = (surface: T) => JSX.Element | Promise<JSX.Element>;
