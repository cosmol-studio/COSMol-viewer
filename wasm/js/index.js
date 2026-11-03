// npm users pass CK objects. Notebook embeds the generated module directly.
import * as ck from "@cosmol-studio/cosmolkit";
import initWasm, { __setCkTypes, __setViewerTypes, Selection, Sphere, Stick, Style } from "./cosmol_viewer_wasm.js";

await Promise.all([ck.default(), initWasm()]);

__setCkTypes(ck.Molecule, ck.Protein, ck.Element);
__setViewerTypes(Selection, Sphere, Stick, Style);

export * from "./cosmol_viewer_wasm.js";
