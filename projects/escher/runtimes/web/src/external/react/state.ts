import type { Dispatch, SetStateAction } from "react";

// TODO: Move to shared/core libs ..
export type MutableState<T> = [
    T | undefined,
    Dispatch<SetStateAction<T | undefined>> | undefined,
];
