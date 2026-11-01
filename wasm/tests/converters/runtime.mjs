import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import test from "node:test";

const pkg = process.env.COSMOL_VIEWER_WASM_CONVERTERS;
const { initSync, Probe } = await import(pathToFileURL(join(pkg, "converters.js")));
initSync({ module: readFileSync(join(pkg, "converters_bg.wasm")) });

test("Serde rejects integer coercion before the native call", () => {
    assert.equal(Probe.byte(255), 255);
    for (const value of [256, -1, 1.5, "1", true, null, undefined, NaN, Infinity]) {
        assert.throws(() => Probe.byte(value), TypeError);
    }
});

test("new structured types share actual input/output declarations and conversions", async () => {
    const value = { count: 3, enabled: true, label: "sample", weights: [1, 65535] };
    assert.deepEqual(Probe.roundTrip(value), value);
    assert.deepEqual(await Probe.asyncRoundTrip(value), value);
    for (const invalid of [
        { ...value, count: 256 }, { ...value, enabled: 1 },
        { ...value, label: 42 }, { ...value, weights: [65536] },
        { ...value, weights: [1.5] }, null,
    ]) {
        assert.throws(() => Probe.roundTrip(invalid), TypeError);
        await assert.rejects(Probe.asyncRoundTrip(invalid), TypeError);
    }
});
