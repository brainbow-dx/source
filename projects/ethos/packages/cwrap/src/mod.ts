
export type Constructor<T = any> = new (...args: any[]) => T;

export type ClassWithStaticProp<P = any> = new (...args: any[]) => { [key: string]: any } & {
    someStaticProp?: P;
};

export function struct<T extends Constructor>(description: string) {
    // Now 'T' is much more readable!
    return function (
        target: T, // The original class constructor (e.g., User or Product)
        context: ClassDecoratorContext<T> // Context object for the class
    ) {
        const className = String(context.name);
        console.log(`[Decorator Setup] Applying 'addDescription' to class '${className}' with description: "${description}"`);

        return class extends target {
            static _description: string = description;

            get _debug() {
                return "TODO";
            }

            constructor(...args: any[]) {
                super(...args);
                console.log(`[Instance Creation] '${className}' instance created. Description: "${description}"`);
            }
        };
    };
}
