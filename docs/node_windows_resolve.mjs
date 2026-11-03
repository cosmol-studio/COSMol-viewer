import { pathToFileURL } from "node:url";

export async function resolve(specifier, context, nextResolve) {
    return nextResolve(/^[A-Za-z]:/.test(specifier)
        ? pathToFileURL(specifier).href : specifier, context);
}
