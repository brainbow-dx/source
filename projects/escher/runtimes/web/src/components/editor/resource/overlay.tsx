// import { EditorContext } from "../editor/sketch/sketch.tsx";
// import { ExcalidrawContext } from "@escher/sketch/components/editor/sketch";
import { useCallback, useContext, useEffect, useRef, useState } from "react";

import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";

// import { EditorContext } from "../editor/sketch/sketch.tsx";
// import { ExcalidrawContext } from "@escher/sketch/components/editor/sketch";

import type { Resource } from "#src/resource.ts";

//---

export interface ResourceDisplayOverlayProps {
    resource?: Resource,
    debugMode?: boolean,
}

export function ResourceDisplayOverlay(props: ResourceDisplayOverlayProps) {
    // const [editor] = useContext(EditorContext);
    // const [excalidrawApi] = useContext(ExcalidrawContext);
    
    if (!props.resource) {
        return (
            <div>
                TODO: Resource not ready ..
            </div>
        )
    }
    
    return (
        <div
            className="resource-overlay"
            data-uuid={props.resource.path}
            data-debug={props.debugMode}
        >
            <div className="tracing-stream">
                TODO
            </div>
            <div className="metadata">
                <a href={props.resource.path.toString()}>
                    {props.resource.path.toString()}
                </a>
                <span title="Resource UUID">
                    UUID:
                    <span style={{userSelect: "text"}}>
                        {
                            // TODO: Get the resource UUID ..
                            props.resource.uuid
                        }
                    </span>
                </span>
            </div>
        </div>
    )
}

export interface ResourceEmbedProps {
    element?: Partial<ExcalidrawElement>;
    title?: string;
    content?: string;
}

export function ResourceEmbed(props: ResourceEmbedProps) {
    const iframe = useRef<HTMLIFrameElement>(null);
    
    const [src, setSrc] = useState(props.element?.link ?? undefined);
    const [title, setTitle] = useState(props.title);
    
    // TODO: Use the "Trusted Types API" to create secure injection templates.
    // Reference: https://developer.mozilla.org/en-US/docs/Web/API/Trusted_Types_API#using_a_csp_to_enforce_trusted_types
    const [content, setContent] = useState(props.content);
    
    const mountResource = useCallback(async () => {
        if (!src) {
            return console.debug(`Resource specifier missing ..`);
        }
        
        try {
            if (/^http/i.test(src)) {
                // const address = iframe.current.contentWindow?.location.href;
                
                // TODO:
                //  - Can we see the page?
                //  - Can we send commands to it?
                //. - Does it attempt to send commands to us?
                
                return console.debug(`Mounted external resource 'TODO' ..`, iframe.current?.contentWindow);
            }
            
            if (/^(file|\.\/\\)/i.test(src)) {
                // const address = iframe.current?.contentWindow?.location.href;
                
                // TODO:
                //  - Can we see the page?
                //  - Can we send commands to it?
                //. - Does it attempt to send commands to us?
                
                return console.debug(`Mounted local (fs) resource ..`, iframe.current?.contentWindow);
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
                
                const getResponse = await fetch(resource.pathname, {
                    method: "GET",
                    headers: {
                        // "Bearer": "TODO",
                        // TODO: Which key to use here?
                        "Accept": "application/json",
                    }
                });
                
                if (![200, 404].find(status => status == headResponse.status)) {
                    throw new Error(`Failed to fetch resource: ${headResponse.statusText}`);
                }
                
                // TODO: Unpack the returned media-type and show it here ..
                // https://www.iana.org/assignments/media-types/application.csv
                console.debug(`Resource Embed HEAD Response:`, headResponse);
                
                // console.log(`Embeds: ${iframe.current.contentDocument?.embeds}`);
            }
        } catch (error) {
            console.error(`Failed to embed resource '${src}':`, error);
            // TODO: Mount error page from html / jsx ..
            setContent(`Failed to embed resource @ '${src}'.`);
        }
    }, [
        title,
        content,
        iframe,
    ]);
        
    return (
        <div className="embed-container">
            <iframe // ref={iframe}
                title={title}
                src={src} // TODO: Src from .. elsewhere?
                srcDoc={content}
                allowFullScreen
                onLoad={_ => {
                    return mountResource();
                }}
                onAbort={event => {
                    console.warn(`Aborted:`, event);
                }}
                style={{
                    overflow: "none",
                }}
            />
        </div>
    )
}
