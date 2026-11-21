import * as ck from "@cosmol-studio/cosmolkit";
import * as cv from "@cosmol-studio/cosmol-viewer";

const status = document.querySelector("#status");
const buttons = [...document.querySelectorAll("[data-mode]")];
const update = document.querySelector("#update");
const highlight = document.querySelector("#oxygen");
const fps = document.querySelector("#fps");
const camera = document.querySelector("#camera");
const screenshot = document.querySelector("#screenshot");
const files = new URL("../../../cosmol_viewer/examples/", import.meta.url);

let viewer;
let scene;
let molecule;
let atoms;
let oxygens;
let mode;
let tinted = false;
let toggleMoleculeColors;
const handles = [];

// Keep the API calls below explicit while releasing WASM handles on demo changes.
function own(handle) {
    handles.push(handle);
    return handle;
}

function dispose() {
    if (viewer) {
        viewer.close();
        viewer.free();
        viewer = undefined;
    }
    while (handles.length) handles.pop().free();
    scene = molecule = atoms = oxygens = undefined;
    toggleMoleculeColors = undefined;
}

async function text(name) {
    const response = await fetch(new URL(name, files));
    if (!response.ok) throw new Error(`Cannot load ${name}: HTTP ${response.status}`);
    return response.text();
}

async function perform(action) {
    for (const control of document.querySelectorAll("button, input")) control.disabled = true;
    status.dataset.error = "false";
    try {
        await action();
    } catch (error) {
        status.dataset.error = "true";
        status.textContent = String(error);
        console.error(error);
    } finally {
        for (const button of buttons) button.disabled = false;
        for (const control of [fps, camera, screenshot]) control.disabled = !viewer;
        const ligandControls = viewer && (mode === "molecule" || mode === "protein");
        update.disabled = !viewer || (!ligandControls && !toggleMoleculeColors);
        highlight.disabled = !ligandControls;
    }
}

function showLigand() {
    const base = own(cv.Style.ballAndStick());
    const style = tinted ? own(base.color(new Uint8Array([60, 210, 220]))) : base;
    scene.set("ligand", molecule, style);
}

function showMoleculeColors(ligand) {
    const xyz = ligand.coordinates3d();
    const [cx, cy, cz] = [0, 1, 2].map(axis => xyz.reduce((sum, p) => sum + p[axis], 0) / xyz.length);
    // Arrange copies in a grid so their colors can be compared side by side.
    const place = (x, y) => own(ligand.with3dCoordinates(
        xyz.map(([px, py, pz]) => [pz - cz + x, py - cy + y, cx - px]),
    ));
    scene.setCameraView(0, 0, 0, 39, new Float32Array([0, 0, 0]), 32);

    const palette = own(cv.MoleculeColors.new({C: new Uint8Array([92, 203, 229])}));
    const defaultColors = own(cv.MoleculeColors.default());
    const yellow = new Uint8Array([255, 209, 102]);
    const pink = new Uint8Array([242, 131, 203]);
    const shared = own(palette.withAtomColors({0: yellow, 3: yellow}));
    const different = own(palette.withAtomColors({0: pink, 3: pink}));
    const sticks = own(cv.Style.stick());
    const highlighted = own(sticks.colors(shared));
    const pinkSticks = own(sticks.colors(different));
    const displays = [
        ["palette", place(-5.5, 4.8), sticks],
        ["shared-a", place(5.5, 4.8), highlighted],
        ["shared-b", place(-5.5, -4.8), highlighted],
        ["different", place(5.5, -4.8), pinkSticks],
    ];
    const applyColors = useBuiltIn => {
        scene.setMoleculePalette(useBuiltIn ? defaultColors : palette);
        for (const [name, molecule, style] of displays) {
            scene.set(name, molecule, useBuiltIn ? sticks : style);
        }
    };
    applyColors(false);
    let useBuiltIn = false;
    return () => {
        useBuiltIn = !useBuiltIn;
        applyColors(useBuiltIn);
        update.textContent = useBuiltIn ? "Apply custom colors" : "Restore built-in colors";
        status.textContent = useBuiltIn ? "Built-in colors restored" : molecularColorsLegend;
    };
}

const molecularColorsLegend = "Top left: scene palette. Top right and bottom left: shared atom colors. Bottom right: separate atom colors.";

async function showExample(nextMode) {
    dispose();
    mode = nextMode;
    tinted = false;
    highlight.checked = false;
    status.textContent = "Loading scene…";
    update.textContent = mode === "colors" ? "Restore built-in colors" : "Update ligand color";
    for (const button of buttons) button.setAttribute("aria-pressed", String(button.dataset.mode === mode));

    // Scientific data belongs to CK; the viewer never creates a molecule wrapper.
    molecule = own(ck.Molecule.fromSdf(await text("6fi1_ligand.sdf")));
    scene = own(cv.Scene.new());
    scene.setBackgroundColor(new Uint8Array([2, 21, 41]));
    if (mode === "colors") {
        toggleMoleculeColors = showMoleculeColors(own(molecule.withKekulizedBonds()));
    } else {
        const selector = own(cv.select(molecule));
        atoms = own(selector.atoms());
        const oxygen = own(ck.Element.fromSymbol("O"));
        oxygens = own(atoms.element(oxygen));
        showLigand();
        // Only camera framing changes; CK coordinates stay unchanged.
        scene.recenterOn(molecule);
        scene.setCameraView(-55, 20, 0, 32, new Float32Array([0, 0, 0]), 18);
    }

    if (mode === "protein") {
        const protein = own(ck.Protein.fromMmcif(await text("6fi1.cif")));
        const cartoon = own(cv.Style.cartoon());
        const rainbow = own(cartoon.rainbowResidues());
        scene.set("protein", protein, rainbow);
        scene.setScale(0.8);
    }

    if (mode === "sidechains") {
        scene.removeShape("ligand");
        const protein = own(ck.Protein.fromMmcif(await text("6fi1.cif")));
        const source = own(cv.select(protein));
        const allAtoms = own(source.atoms());
        // Zero-based CK residue IDs, not PDB sequence numbers.
        const chosen = own(allAtoms.residueIds([10, 11, 12, 13, 14]));
        const sidechains = own(chosen.sidechain());
        const remaining = own(allAtoms.difference(sidechains));
        const sticks = own(cv.Style.stick());
        const red = own(sticks.color(new Uint8Array([255, 68, 68])));
        scene.set("protein", remaining, sticks);
        scene.set("sidechains", sidechains, red);
        scene.recenterOn(protein);
        scene.setScale(0.2);
    }

    if (mode === "residues") {
        scene.removeShape("ligand");
        const protein = own(ck.Protein.fromMmcif(await text("6fi1.cif")));
        const source = own(cv.select(protein));
        const residues = own(source.residues());
        // Zero-based CK residue IDs, not the residue numbers in the CIF.
        const highlighted = own(residues.filterIds([10, 11, 12, 13, 14]));
        const remaining = own(residues.difference(highlighted));
        // Complementary selections share complete-chain Cartoon geometry.
        const cartoon = own(cv.Style.cartoon());
        const gray = own(cartoon.color(new Uint8Array([170, 180, 192])));
        const red = own(cartoon.color(new Uint8Array([255, 68, 68])));
        scene.set("remaining", remaining, gray);
        scene.set("highlighted", highlighted, red);
        scene.recenterOn(protein);
        scene.setScale(0.2);
    }

    viewer = cv.Viewer.new("viewer");
    if (mode === "animation") {
        // Frames share the same CK source and capture independent display state.
        const animation = own(cv.Animation.new(0.06, -1n, true));
        for (let i = 0; i < 32; i++) {
            const frame = scene.cloneScene();
            frame.setScale(1 + 0.25 * Math.sin(i / 32 * 2 * Math.PI));
            animation.addFrame(frame);
            frame.free();
        }
        await viewer.play(animation);
    } else {
        await viewer.render(scene);
    }
    viewer.showFps(fps.checked);
    viewer.showCameraParameters(camera.checked);
    status.textContent = mode === "animation" ? "Animation ready · 32 frames" :
        mode === "colors" ? molecularColorsLegend :
        mode === "residues" ? "Cartoon - residues 10-14 highlighted" :
        mode === "sidechains" ? "Protein sticks - side chains of residues 10-14 highlighted" :
        mode === "protein" ? "Protein + ligand ready" : `Molecule ready · ${atoms.len()} atoms`;
}

for (const button of buttons) {
    button.addEventListener("click", () => perform(() => showExample(button.dataset.mode)));
}

update.addEventListener("click", () => perform(() => {
    if (toggleMoleculeColors) {
        toggleMoleculeColors();
        viewer.update(scene);
        return;
    }
    tinted = !tinted;
    showLigand();
    viewer.update(scene);
    status.textContent = "Scene updated · original CK molecule unchanged";
}));

highlight.addEventListener("change", () => perform(() => {
    if (highlight.checked) {
        const base = own(cv.Style.sphere());
        const style = own(base.scale(0.7));
        scene.set("oxygen", oxygens, style);
    } else {
        scene.removeShape("oxygen");
    }
    viewer.update(scene);
    status.textContent = highlight.checked ? `Highlighted ${oxygens.len()} oxygen atoms` : "Oxygen highlight removed";
}));

fps.addEventListener("change", () => perform(() => viewer.showFps(fps.checked)));
camera.addEventListener("change", () => perform(() => viewer.showCameraParameters(camera.checked)));
screenshot.addEventListener("click", () => perform(async () => {
    const png = await viewer.takeScreenshot();
    const url = URL.createObjectURL(new Blob([png], {type: "image/png"}));
    const link = document.createElement("a");
    link.href = url;
    link.download = `cosmol-${mode}.png`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    status.textContent = "PNG saved";
}));

window.addEventListener("pagehide", dispose);

try {
    await perform(() => showExample("molecule"));
} catch (error) {
    status.dataset.error = "true";
    status.textContent = String(error);
    console.error(error);
}
