import { Probe, type Settings } from "cosmol-viewer-converters";

const value: Settings = { count: 3, enabled: true, label: "sample", weights: [1, 2] };
const returned: Settings = Probe.roundTrip(value);
const asynchronous: Promise<Settings> = Probe.asyncRoundTrip(value);
// @ts-expect-error The structured output is not any.
const wrong: string = returned;
// @ts-expect-error Async structured outputs are also concrete.
const wrongAsync: Promise<string> = Probe.asyncRoundTrip(value);
// @ts-expect-error The same field type governs actual input conversion.
Probe.roundTrip({ ...value, count: "3" });
// @ts-expect-error Arrays retain their element types.
Probe.roundTrip({ ...value, weights: [true] });
void [asynchronous, wrong, wrongAsync];
