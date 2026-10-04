// Real WebGL smoke test, separate from mocked Notebook/Node decoder checks.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const pkg = process.env.COSMOL_VIEWER_WASM_PKG || join(root, "crates/wasm/pkg");
const playwright = process.env.PLAYWRIGHT_MODULE || join(root, "target/browser-check/node_modules/playwright/index.mjs");
const { chromium } = await import(pathToFileURL(playwright));

test("typed JS scenes render/update/screenshot and animations play in a real browser", { timeout: 60000 }, async () => {
    const files = new Set(["cosmol_viewer_wasm.js", "cosmol_viewer_wasm_bg.wasm"]);
    const server = createServer(async (request, response) => {
        const name = request.url.slice(1);
        if (request.url === "/") {
            response.setHeader("Content-Type", "text/html");
            response.end('<!doctype html><html><body><canvas id="mounted" width="128" height="128"></canvas></body></html>');
        } else if (files.has(name)) {
            try {
                response.setHeader("Content-Type", name.endsWith(".wasm") ? "application/wasm" : "text/javascript");
                response.end(await readFile(join(pkg, name)));
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
            const cv = await import("/cosmol_viewer_wasm.js");
            await cv.default();
            const scene = cv.Scene.new();
            scene.addSphere("sphere", new Float32Array([0, 0, 0]), 1);
            scene.setBackgroundColor(new Uint8Array([0, 0, 0]));
            const viewer = await cv.Viewer.render(scene, 128, 128);
            const first = Array.from(await viewer.takeScreenshot());
            scene.setBackgroundColor(new Uint8Array([255, 255, 255]));
            viewer.update(scene);
            viewer.setCameraParameterLogging(false);
            const second = Array.from(await viewer.takeScreenshot());
            const envelope = await viewer.takeScreenshotNotebook();
            const mounted = await cv.Viewer.renderInto("mounted", scene);
            mounted.update(scene);
            await mounted.takeScreenshot();
            mounted.close();
            mounted.free();
            const preservedCanvas = !!document.getElementById("mounted");
            const animation = cv.Animation.new(0.05, -1n, false);
            animation.addFrame(scene);
            scene.setScale(1.5);
            animation.addFrame(scene);
            const playing = await cv.Viewer.play(animation, 128, 128);
            await playing.takeScreenshot();
            const shapes = scene.shapeCount();
            const frames = animation.frameCount();
            playing.close();
            playing.free();
            viewer.close();
            viewer.free();
            animation.free();
            scene.free();
            return { first, second, envelope, shapes, frames, preservedCanvas,
                remainingCanvases: document.querySelectorAll("canvas").length };
        });
        assert.deepEqual(errors, [], "Browser startup/renderer reported errors");
        assert.deepEqual(result.first.slice(0, 8), [137, 80, 78, 71, 13, 10, 26, 10]);
        assert.deepEqual(result.second.slice(0, 8), [137, 80, 78, 71, 13, 10, 26, 10]);
        assert.notDeepEqual(result.first, result.second, "Scene update did not change the screenshot");
        assert.ok(result.envelope.length > 0);
        assert.equal(result.shapes, 1);
        assert.equal(result.frames, 2);
        assert.equal(result.preservedCanvas, true);
        assert.equal(result.remainingCanvases, 1, "close did not remove auto-created canvases");
    } finally {
        if (browser) await browser.close();
        server.closeAllConnections();
        await new Promise(resolve => server.close(resolve));
    }
});
