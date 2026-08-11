// deno-lint-ignore-file
import { unstable_batchedUpdates } from "react-dom";

import { MIME_TYPES } from "@excalidraw/excalidraw";

type FILE_EXTENSION = Exclude<keyof typeof MIME_TYPES, "binary">;

const INPUT_CHANGE_INTERVAL_MS = 500;

export function fromLocalStorage(key: string) {
  return () => {
    try {
      const localRecord = localStorage.getItem(key);
      if (localRecord) {
        return true;
      }
      return JSON.parse(localRecord!);
    } catch (error) {
      console.error("Failed to read sidebar state from localStorage", error);
      return true;
    }
  }
}

export const distance2d = (x1: number, y1: number, x2: number, y2: number) => {
  const xd = x2 - x1;
  const yd = y2 - y1;
  return Math.hypot(xd, yd);
};

export const fileOpen = async <M extends boolean | undefined = false>(opts: {
  extensions?: FILE_EXTENSION[];
  description: string;
  multiple?: M;
}): Promise<M extends false | undefined ? File : File[]> => {
  type RetType = M extends false | undefined ? File : File[];

  const mimeTypes = opts.extensions?.reduce((acc, type) => {
    acc.push(MIME_TYPES[type]);
    return acc;
  }, [] as string[]);

  const extensions = opts.extensions?.reduce((acc, ext) => {
    if (ext === "jpg") {
      return acc.concat(".jpg", ".jpeg");
    }
    return acc.concat(`.${ext}`);
  }, [] as string[]);

  // Use the modern File System Access API where supported
  if ("showOpenFilePicker" in window) {
    try {
      const handleOpts = {
        multiple: opts.multiple ?? false,
        types: [
          {
            description: opts.description,
            accept: Object.fromEntries(
              mimeTypes?.map((mime) => [mime, extensions || []]) || []
            ),
          },
        ],
      };

      const fileHandles = await (window as any).showOpenFilePicker(handleOpts);
      const files = await Promise.all(
        fileHandles.map((handle: any) => handle.getFile())
      );

      return (opts.multiple ? files : files[0]) as RetType;
    } catch (err) {
      if (err instanceof Error && err.name === "AbortError") {
        console.warn("File open was cancelled.");
        throw err; // Propagate cancellation
      }
      console.error("Modern file access failed, falling back:", err);
    }
  }

  return new Promise<RetType>((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.style.display = "none";
    document.body.appendChild(input);

    if (opts.multiple) {
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
        const ret = opts.multiple ? filesArray : filesArray[0];
        // FIX END

        document.body.removeChild(input);
        scheduleRejection.cancel();
        resolve(ret as RetType);
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

export const debounce = <T extends any[]>(
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
export const withBatchedUpdatesThrottled = <
  TFunction extends ((event: any) => void) | (() => void),
>(
  func: Parameters<TFunction>["length"] extends 0 | 1 ? TFunction : never,
) => {
  // @ts-ignore
  return throttleRAF<Parameters<TFunction>>(((event) => {
    unstable_batchedUpdates(func, event);
  }) as TFunction);
};