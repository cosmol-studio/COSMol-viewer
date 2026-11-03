import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import test from "node:test";

const pkg = process.env.COSMOL_VIEWER_WASM_PKG;
assert.ok(pkg, "COSMOL_VIEWER_WASM_PKG must point at the generated package");
// Node consumes local bytes through the raw generated module. The default
// browser npm entry and automatic CK initialization are tested in Chromium.
const binding = await import(pathToFileURL(join(pkg, "cosmol_viewer_wasm.js")));
binding.initSync({ module: readFileSync(join(pkg, "cosmol_viewer_wasm_bg.wasm")) });
const ckTypes = await import(pathToFileURL(join(pkg, "node_modules/@cosmol-studio/cosmolkit/cosmolkit_wasm.js")));
ckTypes.initSync({ module: readFileSync(join(pkg, "node_modules/@cosmol-studio/cosmolkit/cosmolkit_wasm_bg.wasm")) });
binding.__setCkTypes(ckTypes.Molecule, ckTypes.Protein, ckTypes.Element);
binding.__setViewerTypes(binding.Selection, binding.Sphere, binding.Stick, binding.Style);

test("CK selections project molecules and proteins without viewer chemistry wrappers", async () => {
    const ckPath = join(pkg, "node_modules/@cosmol-studio/cosmolkit");
    const ck = await import(pathToFileURL(join(ckPath, "cosmolkit_wasm.js")));
    ck.initSync({module: readFileSync(join(ckPath, "cosmolkit_wasm_bg.wasm"))});
    const source = ck.Molecule.fromSmiles("CO").withAdded3dConformer([[0, 0, 1], [2, 0, 3]]);
    const selector = binding.select(source);
    const atoms = selector.atoms();
    const oxygenElement = ck.Element.fromSymbol("O");
    const oxygen = atoms.element(oxygenElement);
    assert.deepEqual(Array.from(atoms.ids()), [0, 1]);
    assert.deepEqual(Array.from(oxygen.ids()), [1]);
    const scene = binding.Scene.new();
    const balls = binding.Style.ballAndStick();
    const spheres = binding.Style.sphere();
    const scaled = spheres.scale(1.4);
    scene.set("mol", atoms, balls);
    const saved = scene.toJson();
    scene.add(oxygen, scaled);
    scene.recenterOn(source);
    assert.equal(scene.shapeCount(), 2);
    assert.deepEqual(JSON.parse(scene.toJson()).scene_center, [1, 0, 2]);
    const before = scene.toJson();
    assert.throws(() => scene.set("mol", atoms, binding.Style.cartoon()), /support/);
    assert.equal(scene.toJson(), before);
    source.setAtomPosition_(0, [9, 9, 9]);
    const fresh = binding.select(source).atoms();
    scene.set("mol", fresh, balls);
    assert.equal(scene.toJson(), before);
    const other = source.copy();
    scene.set("mol", other, balls);
    assert.notEqual(scene.toJson(), saved);
    const unpositioned = ck.Molecule.fromSmiles("CO");
    const unpositionedAtoms = binding.select(unpositioned).atoms();
    assert.throws(() => scene.add(unpositionedAtoms, balls), /no stored coordinates/);
    assert.equal(unpositioned.coordinates2d, undefined);
    assert.throws(() => binding.select({}), TypeError);
    assert.throws(() => atoms.element({atomicNumber: 8}), TypeError);
    assert.throws(() => selector.residues(), /residues/);
    assert.throws(() => spheres.scale(0), RangeError);
    assert.throws(() => spheres.opacity(NaN), RangeError);
    assert.throws(() => spheres.scale("1.4"), TypeError);
    assert.equal(binding.Molecule, undefined);
    assert.equal(binding.Protein, undefined);

    const root = new URL("../../cosmol_viewer/examples/", import.meta.url);
    const protein = ck.Protein.fromMmcif(readFileSync(new URL("6fi1.cif", root), "utf8"));
    const residues = binding.select(protein).residues();
    const cartoon = binding.Style.cartoon();
    const rainbow = cartoon.rainbowResidues();
    scene.set("protein", residues, rainbow);
    const proteinOxygens = binding.select(protein).atoms().element(oxygenElement);
    scene.add(proteinOxygens, spheres);
    const proteinBefore = scene.toJson();
    protein.translate_([1, 0, 0]);
    scene.set("protein", residues, rainbow);
    assert.equal(scene.toJson(), proteinBefore);
    for (const handle of [proteinOxygens, rainbow, cartoon, residues, protein,
        unpositionedAtoms, unpositioned, fresh, other, scaled, spheres, balls, scene,
        oxygen, oxygenElement, atoms, selector, source]) handle.free();
});


test("add, set, select and cloned scenes share one CK transfer per source", async () => {
    const ckPath = join(pkg, "node_modules/@cosmol-studio/cosmolkit");
    const ck = await import(pathToFileURL(join(ckPath, "cosmolkit_wasm.js")));
    const protein = ck.Protein.fromMmcif(readFileSync(new URL("../../cosmol_viewer/examples/6fi1.cif", import.meta.url), "utf8"));
    const mol = ck.Molecule.fromSmiles("CO").withAdded3dConformer([[0, 0, 0], [2, 0, 0]]);
    let proteinTransfers = 0, moleculeTransfers = 0;
    const proteinTransfer = protein.toTransferBinary.bind(protein);
    const moleculeTransfer = mol.toTransferBinary.bind(mol);
    protein.toTransferBinary = () => { proteinTransfers++; return proteinTransfer(); };
    mol.toTransferBinary = () => { moleculeTransfers++; return moleculeTransfer(); };
    const scene = binding.Scene.new();
    const cartoon = binding.Style.cartoon();
    const surface = binding.Style.surface();
    const balls = binding.Style.ballAndStick();
    scene.add(protein);
    scene.add(protein, surface);
    scene.set("ligand", mol);
    const proteinSelector = binding.select(protein);
    const residues = proteinSelector.residues();
    const moleculeSelector = binding.select(mol);
    const atoms = moleculeSelector.atoms();
    scene.set("protein", residues, cartoon);
    scene.set("ligand", atoms, balls);
    // Passing a selection must not consume it.
    scene.add(residues, cartoon);
    assert.ok(residues.len() > 0);
    const clone = scene.cloneScene();
    scene.free();
    clone.add(protein, cartoon);
    clone.add(mol, balls);
    clone.recenterOn(protein);
    assert.equal(proteinTransfers, 1);
    assert.equal(moleculeTransfers, 1);
    const before = clone.toJson();
    assert.throws(() => clone.set("protein", mol, cartoon), /support/);
    assert.throws(() => clone.add({}, cartoon), TypeError);
    assert.equal(clone.toJson(), before);
    for (const value of [clone, residues, atoms, proteinSelector, moleculeSelector, cartoon, surface, balls, protein, mol]) value.free();
});

test("generated browser API has the declared names and native-only restrictions", () => {
    const handle = binding.Viewer.prototype;
    assert.equal(binding.WebHandle, undefined);
    for (const name of ["update", "dispatch", "cameraParameterLogging", "showFps", "showCameraParameters", "takeScreenshot"]) {
        assert.equal(typeof handle[name], "function", name);
    }
    const rows = JSON.parse(binding.bindingContractJson());
    const wait = rows.find(row => row.semantic_id === "Viewer.keep_alive");
    assert.deepEqual(wait.python.platforms, ["native"]);
    assert.equal(wait.javascript.name, null);
    assert.ok(wait.javascript.unsupported_reason);
    assert.equal(typeof handle.keepAlive, "undefined");
    assert.equal(typeof handle.takeScreenshotNotebook, "undefined");
    const screenshot = rows.find(row => row.semantic_id === "Viewer.take_screenshot");
    assert.equal(screenshot.javascript.notebook_endpoint, null);
    for (const row of rows) {
        if (row.javascript.name) {
            const [owner, method] = row.javascript.name.split(".");
            const type = binding[owner];
            assert.ok(type, owner);
            assert.ok(method ? typeof type[method] === "function" || typeof type.prototype[method] === "function" : typeof type === "function", row.semantic_id);
        }
        if (row.javascript.notebook_endpoint) {
            const [owner, method] = row.javascript.notebook_endpoint.split(".");
            assert.ok(typeof binding[owner][method] === "function" || typeof binding[owner].prototype[method] === "function");
        }
    }
});

test("viewer construction requires an existing canvas and exposes separate render/play", async () => {
    assert.throws(() => binding.Viewer.new("missing-canvas"), /Canvas not found/);
    assert.equal(binding.Viewer.open, undefined);
    assert.equal(binding.Viewer.play, undefined);
    assert.equal(binding.Viewer.openInto, undefined);
    assert.equal(binding.Viewer.playInto, undefined);
    assert.equal(typeof binding.Viewer.prototype.render, "function");
    assert.equal(typeof binding.Viewer.prototype.play, "function");
    await assert.rejects(binding.Viewer.openNotebook("missing-canvas", "invalid"), /decode failed/);
    await assert.rejects(binding.Viewer.playNotebook("missing-canvas", "CMV99:R:AAAA"), /unsupported payload version/);
    await assert.rejects(binding.Viewer.openNotebook("missing-canvas", "CMV1:AAAA"), /unsupported payload version/);
});

test("JS creates and modifies real Scene/Animation state without transport payloads", () => {
    const scene = binding.Scene.new();
    scene.set("sphere", binding.Sphere.new(new Float32Array([0, 0, 0]), 1));
    scene.set("stick", binding.Stick.new(new Float32Array([0, 0, 0]), new Float32Array([1, 0, 0]), 0.1));
    scene.setBackgroundColor(new Uint8Array([0, 0, 0]));
    scene.setTransparentBackground(true);
    scene.setAutoRotate(true, 20);
    scene.setCameraView(0, 0, 0, 35, new Float32Array([0, 0, 0]), 15);
    scene.setDepthCueRange(0.2, 0.8);
    assert.equal(scene.shapeCount(), 2);
    const animation = binding.Animation.new(0.1, -1n, false);
    animation.addFrame(scene);
    scene.setScale(2);
    animation.addFrame(scene);
    animation.setStaticScene(scene);
    animation.setInterval(0.2);
    animation.setLoops(2n);
    animation.setInterpolate(true);
    const state = JSON.parse(animation.toJson());
    assert.equal(state.frames[0].scale, 1);
    assert.equal(state.frames[1].scale, 2);
    assert.equal(state.static_scene.scale, 2);
    assert.equal(state.interval, 200);
    assert.equal(state.loops, 2);
    assert.equal(state.interpolate, true);
    assert.equal(animation.frameCount(), 2);
    assert.equal(scene.shapeCount(), 2);
    const copy = scene.cloneScene();
    copy.setScale(3);
    assert.equal(JSON.parse(scene.toJson()).scale, 2);
    scene.mergeShapes(scene); // Shared/self-alias inputs must not deadlock.
    assert.equal(scene.shapeCount(), 4);
    scene.removeShape("sphere");
    assert.equal(scene.shapeCount(), 3);
    assert.throws(() => scene.removeShape("missing"), /not found/);
    assert.throws(() => scene.recenter(new Float32Array([0, 0])), /three coordinates/);
    assert.throws(() => scene.setDepthCueRange(0.8, 0.2), /Depth cue range/);
    assert.throws(() => scene.setScale(-1), /positive/);
    copy.free();
    animation.free();
    scene.free();
});

test("checked JS values reject coercion and leave native state unchanged", async () => {
    const scene = binding.Scene.new();
    const animation = binding.Animation.new(0.1, -1n, false);
    const beforeScene = scene.toJson();
    const beforeAnimation = animation.toJson();
    for (const invalid of ["2", true, null, undefined, {}, [2]]) {
        assert.throws(() => scene.setScale(invalid), TypeError);
        assert.throws(() => animation.setInterval(invalid), TypeError);

    }
    for (const invalid of [2, true, null, undefined, {}, ["canvas"]]) {
        assert.throws(() => binding.Viewer.new(invalid), TypeError);
    }
    for (const invalid of [0, 1, "false", null, undefined]) {
        assert.throws(() => scene.setTransparentBackground(invalid), TypeError);
        assert.throws(() => animation.setInterpolate(invalid), TypeError);
    }
    for (const invalid of [2, "2", true, 1n << 63n, -(1n << 63n) - 1n]) {
        assert.throws(() => animation.setLoops(invalid), TypeError);
        assert.throws(() => binding.Animation.new(0.1, invalid, false), TypeError);
    }
    for (const invalid of [[255, 0, 0], new Float32Array([255, 0, 0]), "red"]) {
        assert.throws(() => scene.setBackgroundColor(invalid), TypeError);
    }
    for (const invalid of [[0, 0, 0], new Float64Array([0, 0, 0]), "0,0,0"]) {
        assert.throws(() => scene.recenter(invalid), TypeError);
    }
    assert.throws(() => scene.set(12, binding.Sphere.new(new Float32Array([0, 0, 0]), 1)), TypeError);
    assert.throws(() => scene.setAutoRotate(true, "2"), TypeError);
    assert.throws(() => scene.recenter(new Float32Array([0, 0, 0, 1])), /three coordinates/);
    assert.equal(scene.toJson(), beforeScene);
    assert.equal(animation.toJson(), beforeAnimation);
    assert.throws(() => animation.addFrame({}), /Scene/);
    animation.free();
    scene.free();
});

test("default styles, reusable shapes and named replacement share add/set", async () => {
    const ck = await import(pathToFileURL(join(pkg, "node_modules/@cosmol-studio/cosmolkit/cosmolkit_wasm.js")));
    const mol = ck.Molecule.fromSmiles("CO").withAdded3dConformer([[0,0,0], [1,0,0]]);
    const scene = binding.Scene.new(), explicit = binding.Scene.new();
    const style = binding.Style.ballAndStick();
    scene.set("item", mol);
    explicit.set("item", mol, style);
    assert.equal(scene.toJson(), explicit.toJson());
    const sphere = binding.Sphere.new(new Float32Array([0,0,0]), 1);
    const stick = binding.Stick.new(new Float32Array([0,0,0]), new Float32Array([1,0,0]), 0.2);
    scene.set("item", sphere);
    const before = scene.toJson();
    assert.throws(() => scene.set("item", sphere, style), /already carry/);
    assert.equal(scene.toJson(), before);
    scene.set("item", stick);
    scene.add(sphere);
    scene.add(sphere, undefined);
    scene.add(stick, null);
    assert.equal(scene.shapeCount(), 4);
    assert.throws(() => binding.Sphere.new(new Float32Array([0,0]), 1));
    assert.throws(() => binding.Stick.new(new Float32Array([0,0,0]), new Float32Array([1,0,0]), -1));
    for (const name of ["addSphere", "addStick"]) assert.equal(scene[name], undefined);
    for (const value of [scene, explicit, sphere, stick, style, mol]) value.free();
});
