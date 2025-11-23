// deno-lint-ignore-file

// TODO: Move all of the following out to Ethos/Scribe and/or Cwrap.
export interface Layout {
    size: number;
    alignment: number;
}

export class C {
    public readonly layout: Layout = { size: 0, alignment: 0 };
}

export function $struct<T>(path: string = "."): (sourceClass: any) => void {
    return (target?: T) => {
        // console.debug("Target:", target);
        // console.debug("Struct:", path);
    }
}

export function $hidden(value: boolean): (target?: any) => void {
    return (target?: any) => {
        // console.debug("Target:", target);
        // console.debug("Hidden:", value);
    }
}

export function $locked(value: boolean): (target?: any) => void {
    return (target?: any) => {
        // console.debug("Target:", target);
        // console.debug("Locked:", value);
    }
}

export function $method(arg1?: boolean, ...argz: any[]): (target?: any) => void {
    return (target?: any) => {
        // console.debug("Target:", target);
        // console.debug("Method:", arg1, argz);
    }
}
