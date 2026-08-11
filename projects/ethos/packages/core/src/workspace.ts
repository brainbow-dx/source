
export interface WorkspaceOptions {
    outputDir?: string,
}

export class Workspace {
    #resources = new Map<string, unknown>();
    
    constructor(
        public readonly root?: URL | string,
        public readonly options?: WorkspaceOptions,
    ) {
        //..
    }
    
    // deno-lint-ignore require-await
    public async findResource(): Promise<unknown> {
        return undefined;
    }
}

