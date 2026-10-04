import { Viewer, Scene, Animation, bindingContractJson } from "cosmol-viewer-generated";

const scene: Scene = Scene.new();
scene.addSphere("sphere", new Float32Array([0, 0, 0]), 1);
scene.setBackgroundColor(new Uint8Array([255, 255, 255]));
const animation: Animation = Animation.new(0.1, -1n, true);
animation.addFrame(scene);
animation.setStaticScene(scene);
const rendered: Promise<Viewer> = Viewer.render(scene, 800, 600);
const playing: Promise<Viewer> = Viewer.play(animation, 800, 600);
const mounted: Promise<Viewer> = Viewer.renderInto("canvas-id", scene);
const handle: Viewer = await rendered;
const updated: void = handle.update(scene);
const logged: void = handle.setCameraParameterLogging(true);
const screenshot: Promise<Uint8Array> = handle.takeScreenshot();
const contract: string = bindingContractJson();
// Internal notebook paths remain separate from the public typed API.
const notebook: Promise<Viewer> = Viewer.renderNotebook("canvas-id", "scene-payload");
const submitted: void = handle.dispatch("command-payload");
const envelope: Promise<string> = handle.takeScreenshotNotebook();
// @ts-expect-error Native lifecycle methods must not appear in browser declarations.
handle.keepAlive();
// @ts-expect-error Normal JS render does not accept notebook transport payloads.
Viewer.render("compressed-scene", 800, 600);
// @ts-expect-error Normal JS update takes a Scene, not a transport payload.
handle.update("compressed-scene");
// @ts-expect-error Animation frames must be typed scenes.
animation.addFrame("compressed-scene");
void [playing, mounted, updated, logged, screenshot, contract, notebook, submitted, envelope];
