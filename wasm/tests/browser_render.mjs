// Real WebGL smoke test, separate from mocked Notebook/Node decoder checks.
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";

const root = fileURLToPath(new URL("../../", import.meta.url));
const pkg = process.env.COSMOL_VIEWER_WASM_PKG || join(root, "wasm/pkg");
const playwright = process.env.PLAYWRIGHT_MODULE || join(root, "target/browser-check/node_modules/playwright/index.mjs");
const { chromium } = await import(pathToFileURL(playwright));

test("typed JS scenes render/update/screenshot and animations play in a real browser", { timeout: 60000 }, async () => {
    const files = new Map([
        ["index.js", join(pkg, "index.js")],
        ["cosmol_viewer_wasm.js", join(pkg, "cosmol_viewer_wasm.js")],
        ["cosmol_viewer_wasm_bg.wasm", join(pkg, "cosmol_viewer_wasm_bg.wasm")],
        ["ck/cosmolkit_wasm.js", join(pkg, "node_modules/@cosmol-studio/cosmolkit/cosmolkit_wasm.js")],
        ["ck/cosmolkit_wasm_bg.wasm", join(pkg, "node_modules/@cosmol-studio/cosmolkit/cosmolkit_wasm_bg.wasm")],
    ]);
    const ckRoot = join(pkg, "node_modules/@cosmol-studio/cosmolkit");
    for (const file of await readdir(join(ckRoot, "snippets"), { recursive: true, withFileTypes: true })) {
        if (file.isFile()) {
            const path = join(file.parentPath, file.name);
            const name = path.slice(ckRoot.length + 1).replaceAll("\\", "/");
            files.set(`ck/${name}`, path);
        }
    }
    const loads = new Map();
    const server = createServer(async (request, response) => {
        const name = request.url.slice(1);
        if (request.url === "/") {
            response.setHeader("Content-Type", "text/html");
            response.end('<!doctype html><html><head><script type="importmap">{"imports":{"@cosmol-studio/cosmol-viewer":"/index.js","@cosmol-studio/cosmolkit":"/ck/cosmolkit_wasm.js"}}</script></head><body><section id="viewport"><canvas id="viewer" width="128" height="128"></canvas></section><canvas id="mounted" width="128" height="128"></canvas><canvas id="animation" width="128" height="128"></canvas></body></html>');
        } else if (files.has(name)) {
            try {
                response.setHeader("Content-Type", name.endsWith(".wasm") ? "application/wasm" : "text/javascript");
                loads.set(name, (loads.get(name) || 0) + 1);
                response.end(await readFile(files.get(name)));
            } catch (error) { response.writeHead(500).end(String(error)); }
        } else { response.writeHead(404).end(); }
    });
    await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
    let browser;
    try {
        browser = await chromium.launch({
            headless: true,
            executablePath: process.env.BROWSER_EXECUTABLE,
            args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
        });
        const page = await browser.newPage();
        const errors = [];
        page.on("pageerror", error => errors.push(String(error)));
        page.on("dialog", dialog => { errors.push(dialog.message()); dialog.dismiss(); });
        await page.goto(`http://127.0.0.1:${server.address().port}/`);
        const result = await page.evaluate(async () => {
            const cv = await import("@cosmol-studio/cosmol-viewer");
            const ck = await import("@cosmol-studio/cosmolkit");
            // No init calls or manual constructor registration in the consumer.
            const importedAgain = cv === await import("@cosmol-studio/cosmol-viewer");
            const hasDefaultInit = "default" in cv;
            const molecule = ck.Molecule.fromSmiles("CO").withAdded3dConformer([[0, 0, 0], [2, 0, 0]]);
            const molecularScene = cv.Scene.new();
            molecularScene.add(molecule);
            const molecularShapes = molecularScene.shapeCount();
            molecularScene.free();
            molecule.free();
            const scene = cv.Scene.new();
            scene.set("sphere", cv.Sphere.new(new Float32Array([0, 0, 0]), 1));
            scene.setBackgroundColor(new Uint8Array([0, 0, 0]));
            const viewer = cv.Viewer.new("viewer");
            let beforeRender;
            try { await viewer.takeScreenshot(); } catch (error) { beforeRender = String(error); }
            let invalidDispatch;
            try { viewer.dispatch("invalid-payload"); } catch (error) { invalidDispatch = String(error); }
            const starting = viewer.render(scene);
            // WebGL startup can finish synchronously. A second call either
            // queues a scene or rejects while initialization is still pending.
            const duplicateStart = await viewer.render(scene).then(() => "queued", error => String(error));
            await starting;
            const first = Array.from(await viewer.takeScreenshot());
            scene.setBackgroundColor(new Uint8Array([255, 255, 255]));
            viewer.update(scene);
            viewer.cameraParameterLogging(false);
            const second = Array.from(await viewer.takeScreenshot());
            viewer.showCameraParameters(true);
            const cameraOverlay = Array.from(await viewer.takeScreenshot());
            viewer.showFps(true);
            const combinedOverlay = Array.from(await viewer.takeScreenshot());
            // Enabling diagnostics must not turn a static scene into a render loop.
            const canvas = document.querySelector("canvas:not(#mounted)");
            const gl = canvas.getContext("webgl2");
            let draws = 0;
            const draw = gl.drawElements.bind(gl);
            gl.drawElements = (...args) => { draws++; return draw(...args); };
            await new Promise(resolve => setTimeout(resolve, 300));
            const settledDraws = draws;
            await new Promise(resolve => setTimeout(resolve, 300));
            const idleDraws = draws - settledDraws;
            // The single idle deadline must update the readout to zero, then stop.
            await new Promise(resolve => setTimeout(resolve, 1200));
            const zeroOverlay = Array.from(await viewer.takeScreenshot());
            await new Promise(resolve => setTimeout(resolve, 300));
            const afterZeroDraws = draws;
            await new Promise(resolve => setTimeout(resolve, 1200));
            const laterIdleDraws = draws - afterZeroDraws;
            const stillZeroOverlay = Array.from(await viewer.takeScreenshot());
            viewer.update(scene);
            const resumedOverlay = Array.from(await viewer.takeScreenshot());
            gl.drawElements = draw;
            viewer.showCameraParameters(false);
            viewer.showFps(false);
            const hiddenOverlay = Array.from(await viewer.takeScreenshot());
            const concurrent = await Promise.all([
                viewer.takeScreenshot(), viewer.takeScreenshot(), viewer.takeScreenshot(),
            ]);
            // Queue more commands than the App's per-frame budget; no browser
            // polling loop should be necessary to reach the final screenshot.
            for (let i = 0; i < 12; i++) viewer.cameraParameterLogging(false);
            await viewer.takeScreenshot();
            // Let the first async request enqueue before a later scene update.
            const beforeUpdate = viewer.takeScreenshot();
            await Promise.resolve();
            scene.setBackgroundColor(new Uint8Array([0, 0, 0]));
            viewer.update(scene);
            const afterUpdate = viewer.takeScreenshot();
            const barrier = await Promise.all([beforeUpdate, afterUpdate]);
            const mounted = cv.Viewer.new("mounted");
            await mounted.render(scene);
            mounted.update(scene);
            await mounted.takeScreenshot();
            mounted.close();
            mounted.free();
            const preservedCanvas = !!document.getElementById("mounted");
            const animation = cv.Animation.new(0.05, -1n, false);
            animation.addFrame(scene);
            scene.setScale(1.5);
            animation.addFrame(scene);
            const playing = cv.Viewer.new("animation");
            const empty = cv.Animation.new(0.05, -1n, false);
            const emptyPlay = await playing.play(empty).then(() => "unexpected success", error => String(error));
            empty.free();
            await playing.play(animation);
            await playing.takeScreenshot();
            await playing.render(scene);
            const staticAfterPlay = Array.from(await playing.takeScreenshot());
            await playing.play(animation);
            await playing.takeScreenshot();
            await playing.render(scene);
            const staticAgain = Array.from(await playing.takeScreenshot());
            const shapes = scene.shapeCount();
            const frames = animation.frameCount();
            playing.close();
            const renderAfterClose = await playing.render(scene).then(() => "unexpected success", error => String(error));
            const playAfterClose = await playing.play(animation).then(() => "unexpected success", error => String(error));
            playing.free();
            const pending = viewer.takeScreenshot();
            await Promise.resolve();
            viewer.close();
            const closedWait = await pending.then(() => "unexpected success", error => String(error));
            let closedUpdate;
            try { viewer.update(scene); } catch (error) { closedUpdate = String(error); }
            viewer.free();
            animation.free();
            scene.free();
            return { importedAgain, hasDefaultInit, molecularShapes, beforeRender, invalidDispatch, duplicateStart, emptyPlay, renderAfterClose, playAfterClose, staticAfterPlay, staticAgain, first, second, cameraOverlay, combinedOverlay, zeroOverlay,
                stillZeroOverlay, resumedOverlay, hiddenOverlay, idleDraws, laterIdleDraws, shapes, frames, preservedCanvas,
                concurrent: concurrent.map(png => Array.from(png.slice(0, 8))),
                barrier: barrier.map(png => Array.from(png)), closedWait, closedUpdate,
                viewerParent: document.getElementById("viewer")?.parentElement.id,
                remainingCanvases: document.querySelectorAll("canvas").length };
        });
        assert.deepEqual(errors, [], "Browser startup/renderer reported errors");
        assert.equal(result.importedAgain, true);
        assert.equal(result.hasDefaultInit, false);
        assert.equal(result.molecularShapes, 1, "CK molecule must work immediately after importing Viewer");
        assert.equal(loads.get("cosmol_viewer_wasm_bg.wasm"), 1);
        assert.equal(loads.get("ck/cosmolkit_wasm_bg.wasm"), 1);
        assert.deepEqual(result.first.slice(0, 8), [137, 80, 78, 71, 13, 10, 26, 10]);
        assert.deepEqual(result.second.slice(0, 8), [137, 80, 78, 71, 13, 10, 26, 10]);
        assert.notDeepEqual(result.first, result.second, "Scene update did not change the screenshot");
        assert.notDeepEqual(result.second, result.cameraOverlay, "Camera overlay was not drawn");
        assert.notDeepEqual(result.cameraOverlay, result.combinedOverlay, "FPS overlay was not drawn");
        assert.deepEqual(result.hiddenOverlay, result.second, "Hiding overlays did not restore the scene");
        assert.equal(result.idleDraws, 0, "Diagnostics triggered continuous redraw of an idle scene");
        assert.notDeepEqual(result.zeroOverlay, result.combinedOverlay, "Idle FPS readout was not updated");
        assert.deepEqual(result.zeroOverlay, result.stillZeroOverlay, "Screenshots restarted idle FPS statistics");
        assert.equal(result.laterIdleDraws, 0, "Zero FPS repaint renewed the idle timer");
        assert.notDeepEqual(result.resumedOverlay, result.zeroOverlay, "Scene update did not resume FPS statistics");
        assert.deepEqual(result.concurrent, Array(3).fill([137, 80, 78, 71, 13, 10, 26, 10]));
        assert.deepEqual(result.barrier[0], result.second, "Later update changed an earlier screenshot");
        assert.notDeepEqual(result.barrier[0], result.barrier[1], "FIFO screenshot barrier did not preserve scene order");
        assert.match(result.closedWait, /closed/i);
        assert.match(result.closedUpdate, /closed/i);
        assert.equal(result.shapes, 1);
        assert.equal(result.frames, 2);
        assert.match(result.beforeRender, /before app initialization/);
        assert.match(result.invalidDispatch, /decode failed/);
        assert.match(result.duplicateStart, /^(queued|.*already in progress.*)$/);
        assert.match(result.emptyPlay, /at least one frame/);
        assert.match(result.renderAfterClose, /closed/i);
        assert.match(result.playAfterClose, /closed/i);
        assert.deepEqual(result.staticAfterPlay, result.staticAgain, "Switching playback back to a scene is not repeatable");
        assert.equal(result.preservedCanvas, true);
        assert.equal(result.viewerParent, "viewport", "Viewer moved or removed the page canvas");
        assert.equal(result.remainingCanvases, 3, "close must preserve caller-owned canvases");
    } finally {
        if (browser) await browser.close();
        server.closeAllConnections();
        await new Promise(resolve => server.close(resolve));
    }
});
