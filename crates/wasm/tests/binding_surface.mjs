import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import test from "node:test";

const pkg = process.env.COSMOL_VIEWER_WASM_PKG;
assert.ok(pkg, "COSMOL_VIEWER_WASM_PKG must point at the generated package");
const binding = await import(pathToFileURL(join(pkg, "cosmol_viewer_wasm.js")));
binding.initSync({ module: readFileSync(join(pkg, "cosmol_viewer_wasm_bg.wasm")) });

test("generated browser API has the declared names and native-only restrictions", () => {
    const handle = binding.Viewer.new();
    assert.equal(binding.WebHandle, undefined);
    for (const name of ["update", "dispatch", "setCameraParameterLogging", "takeScreenshot"]) {
        assert.equal(typeof handle[name], "function", name);
    }
    const rows = JSON.parse(binding.bindingContractJson());
    const wait = rows.find(row => row.semantic_id === "Viewer.keep_alive");
    assert.deepEqual(wait.python.platforms, ["native"]);
    assert.equal(wait.javascript.name, null);
    assert.ok(wait.javascript.unsupported_reason);
    assert.equal(typeof handle.keepAlive, "undefined");
    for (const row of rows) {
        if (row.javascript.name) {
            const [owner, method] = row.javascript.name.split(".");
            const type = binding[owner];
            assert.ok(type, owner);
            assert.ok(typeof type[method] === "function" || typeof type.prototype[method] === "function", row.semantic_id);
        }
        if (row.javascript.notebook_endpoint) {
            const [owner, method] = row.javascript.notebook_endpoint.split(".");
            assert.ok(typeof binding[owner][method] === "function" || typeof binding[owner].prototype[method] === "function");
        }
    }
    assert.throws(() => handle.dispatch("invalid-payload"));
    handle.free();
});

test("async browser initialization reports a missing DOM canvas", async () => {
    const handle = binding.Viewer.new();
    const scene = binding.Scene.new();
    const animation = binding.Animation.new(0.1, 1n, false);
    await assert.rejects(binding.Viewer.renderInto("missing-canvas", scene), /Canvas not found/);
    await assert.rejects(binding.Viewer.playInto("missing-canvas", animation), /Canvas not found/);
    await assert.rejects(binding.Viewer.render(scene, 100, 100), /document is unavailable/);
    await assert.rejects(binding.Viewer.play(animation, 100, 100), /document is unavailable/);
    // Async failures must neither consume nor invalidate borrowed JS objects.
    assert.equal(scene.shapeCount(), 0);
    assert.equal(animation.frameCount(), 0);
    await assert.rejects(handle.takeScreenshot(), /before app initialization/);
    await assert.rejects(handle.takeScreenshotNotebook(), /before app initialization/);
    assert.throws(() => handle.update(scene), /before app initialization/);
    assert.throws(() => handle.setCameraParameterLogging(true), /before app initialization/);
    assert.equal(scene.shapeCount(), 0);
    await assert.rejects(binding.Viewer.renderNotebook("missing-canvas", "invalid"), /decode failed/);
    await assert.rejects(binding.Viewer.playNotebook("missing-canvas", "CMV99:invalid"), /unsupported animation payload/);
    scene.free();
    animation.free();
    handle.free();
});

test("JS creates and modifies real Scene/Animation state without transport payloads", () => {
    const scene = binding.Scene.new();
    scene.addSphere("sphere", new Float32Array([0, 0, 0]), 1);
    scene.addStick("stick", new Float32Array([0, 0, 0]), new Float32Array([1, 0, 0]), 0.1);
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
