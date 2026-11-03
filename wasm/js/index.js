// npm users pass CK objects. Notebook embeds the generated module directly.
import { Molecule, Protein, Element } from "@cosmol-studio/cosmolkit";
import initialize, { initSync as initializeSync, __setCkTypes, __setViewerTypes, Selection, Sphere, Stick, Style } from "./cosmol_viewer_wasm.js";

export * from "./cosmol_viewer_wasm.js";

export function initSync(...args) {
    const wasm = initializeSync(...args);
    __setCkTypes(Molecule, Protein, Element);
    __setViewerTypes(Selection, Sphere, Stick, Style);
    return wasm;
}

export default async function init(...args) {
    const wasm = await initialize(...args);
    __setCkTypes(Molecule, Protein, Element);
    __setViewerTypes(Selection, Sphere, Stick, Style);
    return wasm;
}
