// deno-lint-ignore-file
import { unstable_batchedUpdates } from "react-dom";

import { loadSceneOrLibraryFromBlob, MIME_TYPES } from "@excalidraw/excalidraw";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";
import type { ImportedLibraryData } from "@excalidraw/excalidraw/data/types";

type FILE_EXTENSION = Exclude<keyof typeof MIME_TYPES, "binary">;

const INPUT_CHANGE_INTERVAL_MS = 500;

export function useLocalStorage(key: string) {
    return [fromLocalStorage(key), setLocalStorage(key)];
}

export function fromLocalStorage(key: string) {
    try {
        return JSON.parse(localStorage.getItem(key)!);
    } catch (error) {
        console.error("Failed to read sidebar state from localStorage", error);
        return undefined;
    }
}

export function setLocalStorage(key: string) {
    return (value: any) => {
        try {
            localStorage.setItem(key, JSON.stringify(value));
        } catch (error) {
            console.error("Failed to read sidebar state from localStorage", error);
        }
    }
}

export async function loadSceneOrLibrary(excalidraw: ExcalidrawImperativeAPI) {
    // TODO: Wtf??
    const file: any = await openResource({ description: "Excalidraw or library file" });
    const contents = await loadSceneOrLibraryFromBlob(file, null, null);
    if (contents.type === MIME_TYPES.excalidraw) {
        const sceneData = contents.data;
        if (sceneData.appState) {
            // TODO: Allow the user to set a preferred theme in settings.
            const currentAppState = excalidraw.getAppState();
            sceneData.appState.theme = currentAppState.theme;
        }
        excalidraw.updateScene(sceneData);
    } else if (contents.type === MIME_TYPES.excalidrawlib) {
        excalidraw.updateLibrary({
            libraryItems: (contents.data as ImportedLibraryData).libraryItems!,
            openLibraryMenu: true,
        });
    }
};

export const distance2d = (x1: number, y1: number, x2: number, y2: number) => {
    const xd = x2 - x1;
    const yd = y2 - y1;
    return Math.hypot(xd, yd);
};

// TODO: Move the following to src/workspace.ts
export const WORKSPACE_HANDLE_NAME_KEY = "workspace-handle-name";

export async function getWorkspaceHandle() {
    try {
        const storedHandleName = localStorage.getItem(WORKSPACE_HANDLE_NAME_KEY);
        if (storedHandleName) {
            const rootDir = await navigator.storage.getDirectory();
            const handle = await rootDir.getFileHandle(storedHandleName);
            
            const permissionStatus = await (handle as any).queryPermission({ mode: 'read' });
            switch (permissionStatus) {
            case "granted":
                return handle;
            }
        }
        
        const handle = await (window as any).showDirectoryPicker();
        localStorage.setItem(WORKSPACE_HANDLE_NAME_KEY, handle.name);

        return handle;
    } catch (e) {
        console.error('Failed to retrieve stored handle:', e);
    }
}

interface OpenResourceOptions {
    extensions?: FILE_EXTENSION[];
    description: string;
    multiple?: boolean;
}

/**
 * Represents a File value or set of File[] values. 
 * Can be `false` on failure (for now) ..
 * 
 * @todo Return specific errors!
 */
type OpenResourceResponse = File | File[] | false | undefined;

export async function openResource(options: OpenResourceOptions): Promise<OpenResourceResponse> {
    // TODO: Move this out to the actual signature ..
    const mimeTypes = options.extensions?.reduce((acceptMimetypes, type) => {
        acceptMimetypes.push(MIME_TYPES[type]);
        return acceptMimetypes;
    }, [] as string[]);

    const extensions = options.extensions?.reduce((acc, ext) => {
        if (ext === "jpg") {
            return acc.concat(".jpg", ".jpeg");
        }
        return acc.concat(`.${ext}`);
    }, [] as string[]);

    // Use the modern File System Access API where supported
    if ("showOpenFilePicker" in window) {
        try {
            const resourceSelectOptions = {
                multiple: options.multiple ?? false,
                types: [
                    {
                        description: options.description,
                        accept: Object.fromEntries(
                            mimeTypes?.map((mime) => [mime, extensions || []]) || []
                        ),
                    },
                ],
            };

            const fileHandles = await (window as any).showOpenFilePicker(resourceSelectOptions);
            const files = await Promise.all(
                fileHandles.map((handle: any) => handle.getFile())
            );

            return (options.multiple ? files : files[0]) as OpenResourceResponse;
        } catch (err) {
            if (err instanceof Error && err.name === "AbortError") {
                console.warn("File open was cancelled.");
                throw err; // Propagate cancellation
            }
            console.error("Modern file access failed, falling back:", err);
        }
    }

    return new Promise<OpenResourceResponse>((resolve, reject) => {
        const input = document.createElement("input");
        input.type = "file";
        input.style.display = "none";
        document.body.appendChild(input);

        if (options.multiple) {
            input.multiple = true;
        }

        if (mimeTypes?.length || extensions?.length) {
            input.accept = [...(mimeTypes || []), ...(extensions || [])].join(",");
        }

        // Set up cleanup and rejection handlers
        const scheduleRejection = debounce(() => {
            document.body.removeChild(input);
            reject(new Error("File open cancelled or timed out."));
        }, INPUT_CHANGE_INTERVAL_MS);

        // Watch for a file to be selected
        const changeHandler = () => {
            if (input.files?.length) {
                // FIX START: Convert FileList to an array using Array.from()
                const filesArray = Array.from(input.files);
                const ret = options.multiple ? filesArray : filesArray[0];
                // FIX END

                document.body.removeChild(input);
                scheduleRejection.cancel();
                resolve(ret as OpenResourceResponse);
            }
        };
        input.addEventListener("change", changeHandler, { once: true });

        // Click the input to open the file picker
        input.click();

        // In case the user cancels the picker, listen for focus events to trigger cleanup
        const focusHandler = () => {
            window.removeEventListener("focus", focusHandler);
            // Give the change event a chance to fire
            setTimeout(() => {
                if (input.files?.length === 0) {
                    scheduleRejection();
                }
            }, 50);
        };
        window.addEventListener("focus", focusHandler);
    });
};

export const debounce: any = <T extends any[]>(
    fn: (...args: T) => void,
    timeout: number,
) => {
    let handle = 0;
    let lastArgs: T | null = null;
    const ret = (...args: T) => {
        lastArgs = args;
        clearTimeout(handle);
        handle = window.setTimeout(() => {
            lastArgs = null;
            fn(...args);
        }, timeout);
    };
    ret.flush = () => {
        clearTimeout(handle);
        if (lastArgs) {
            const _lastArgs = lastArgs;
            lastArgs = null;
            fn(..._lastArgs);
        }
    };
    ret.cancel = () => {
        lastArgs = null;
        clearTimeout(handle);
    };
    return ret;
};

export const withBatchedUpdates = <
    TFunction extends ((event: any) => void) | (() => void),
>(
    func: Parameters<TFunction>["length"] extends 0 | 1 ? TFunction : never,
) =>
    ((event) => {
        unstable_batchedUpdates(func as TFunction, event);
    }) as TFunction;

/**
 * barches React state updates and throttles the calls to a single call per
 * animation frame
 */
export const withBatchedUpdatesThrottled = <TFunction extends ((event: any) => void) | (() => void)>(
    func: Parameters<TFunction>["length"] extends 0 | 1 ? TFunction : never,
): void => {
    // @ts-ignore
    return throttleRAF<Parameters<TFunction>>(((event) => {
        unstable_batchedUpdates(func, event);
    }) as TFunction);
};