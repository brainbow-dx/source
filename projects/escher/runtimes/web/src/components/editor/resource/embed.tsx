import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";

import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";

/**
 * The kind of resource we're working with.
 * 
 * @todo Move this to the resource model.
 */
export enum ResourceEmbedKind {
    workspace = "workspace",
    external = "external",
    comment = "comment",
    command = "command",
}

/**
 * TODO
 */
export interface ResourceEmbedProps {
    kind?: ResourceEmbedKind;
    element?: Partial<ExcalidrawElement>;
    title?: string | null;
    content?: string;
    onLoad?: (event: React.SyntheticEvent) => void;
    onAbort?: (event: React.SyntheticEvent) => void;
}

export const ResourceEmbedContext = createContext<ResourceEmbedProps>({
    //..
});

export function ResourceEmbed(props: ResourceEmbedProps) {
    const embedSrc = props.element?.link;
    const embedRef = useRef(null);
    
    const [kind, setKind] = useState<ResourceEmbedKind | undefined>(undefined);
    const [title, setTitle] = useState<string | undefined>(undefined);
    const [content, setContent] = useState<string | undefined>(undefined);
    
    // TODO: Use an object we can use to build the final permissions string.
    const [permissions, setPermissions] = useState(`fullscreen; bluetooth; gamepad;`);
    
    const reloadContent = useCallback(async () => {
        console.debug(`Embed Kind:`, kind);
        
        if (embedSrc && embedSrc.length > 0) {
            if (/^(http|https):\/\//i.test(embedSrc)) {
                setKind(ResourceEmbedKind.external);
            }
            else if (/^(\.\/|\/)/i.test(embedSrc)) {
                setKind(ResourceEmbedKind.workspace);
            }
            else if (/^#/i.test(embedSrc)) {
                setKind(ResourceEmbedKind.comment);
            }
            else if (/^$/i.test(embedSrc)) {
                setKind(ResourceEmbedKind.command);
            }
            
            const content = await fetchEmbedContent(embedSrc);
            if (content && content.length > 0) {
                setContent(content);
            }
        }
    }, [
        embedSrc,
    ]);
    
    useEffect(() => void reloadContent(), [
        reloadContent,
        embedSrc,
    ]);
    
    if (content && content.length > 0) {
        return (
            <dialog role="directory">
                <iframe
                    ref={embedRef}
                    title={title}
                    srcDoc={content}
                    allow={permissions}
                    className="content"
                    onLoad={props.onLoad}
                    onAbort={props.onAbort}
                    style={{
                        overflow: "none",
                    }}
                />
            </dialog>
        )
    }
    
    return (
        // External resources use `fencedframe` for added "security".
        // Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/fencedframe
        <dialog role="container" className="embed" ref={embedRef}>
            {kind == ResourceEmbedKind.external && (
                <iframe
                    ref={embedRef}
                    title={title}
                    src={embedSrc ?? undefined} // TODO: Src from .. elsewhere?
                    className="external"
                    allow={permissions}
                    onLoad={props.onLoad}
                    onAbort={props.onAbort}
                />
            )}
            {kind == ResourceEmbedKind.workspace && (
                <object
                    ref={embedRef}
                    title={title}
                    data={embedSrc ?? undefined}
                    className="workspace"
                    onLoad={props.onLoad}
                    onAbort={props.onAbort}
                >
                    TODO: Fallback embed content ..
                </object>
            )}
        </dialog>
    )
}

export async function fetchEmbedContent(src: string) {
    if (/^http/ig.test(src)) {
        // const address = iframe.current.contentWindow?.location.href;
        
        // TODO:
        //  - Can we see the page?
        //  - Can we send commands to it?
        //. - Does it attempt to send commands to us?
    }
    
    if (/^(file|\.\/\\)/ig.test(src)) {
        // const address = iframe.current?.contentWindow?.location.href;
        
        // TODO:
        //  - Can we see the page?
        //  - Can we send commands to it?
        //. - Does it attempt to send commands to us?
    }
    
    const resource = URL.parse(src, globalThis.location.toString());
    if (resource) {
        const headResponse = await fetch(resource.pathname, {
            // TODO: Construct an api call.
            method: "HEAD",
            headers: {
                // "Bearer": "TODO",
            }
        });
        
        if (![200, 404].find(status => status == headResponse.status)) {
            throw new Error(`Failed to fetch resource: ${headResponse.statusText}`);
        }
        
        // TODO: Get returned mimetype + metadata ..
        
        // const getResponse = await fetch(resource.pathname, {
        //     method: "GET",
        //     headers: {
        //         // "Bearer": "TODO",
        //         // TODO: Which key to use here?
        //         "Accept": "application/json",
        //     }
        // });
        
        // if (![200, 404].find(status => status == headResponse.status)) {
        //     throw new Error(`Failed to fetch resource: ${headResponse.statusText}`);
        // }
        
        if (headResponse.headers.get(`X`)) {
            return void console.debug(``);
        }
        
        // TODO: Unpack the returned media-type and show it here ..
        // https://www.iana.org/assignments/media-types/application.csv
        // console.debug(`Resource Embed HEAD Response:`, headResponse);
        
        if (headResponse.headers.get(`X-Resource-Meta`) === "v0") {
            return await headResponse.json();
        }
        
        // console.log(`Embeds: ${iframe.current.contentDocument?.embeds}`);
        
        return await headResponse.text();
    }
}
