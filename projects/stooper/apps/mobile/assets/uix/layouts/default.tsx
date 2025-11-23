/**
 * TODO
 */
export interface WindowProps {
    title: string,
    // deno-lint-ignore no-explicit-any
    children?: any,
}

/**
 * @todo Move this to a different place.
 */
export function Window(_props: WindowProps) {
    return null;
}

/**
 * @returns HTML Document Tree
 */
export function web() {
    return (
        <html>
            <head>
                <title>Stooper</title>
            </head>
            <body>
                <slot name="main" />
            </body>
        </html>
    )
}

export function desktop(_surface: object) {
    return (
        <Window title="Stooper">
            <head>
                <title>TODO</title>
            </head>
            <body>
                <main>
                    <slot name="main" />
                </main>
            </body>
        </Window>
    )
}
