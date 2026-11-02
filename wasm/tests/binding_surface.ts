import { Viewer, Scene, Animation, bindingContractJson } from "cosmol-viewer-generated";
import { select, Style, Selection, Selector } from "cosmol-viewer-generated";
import type { Molecule, Protein, Element } from "@cosmol-studio/cosmolkit";

declare const molecule: Molecule;
declare const protein: Protein;
declare const oxygen: Element;
const selectedSource: Selector = select(molecule);
const selectedAtoms: Selection = selectedSource.atoms().element(oxygen);
const selectedIds: Uint32Array = selectedAtoms.ids();
const scientificScene: Scene = Scene.new();
const shown: void = scientificScene.show(molecule, selectedAtoms, Style.sphere().scale(1.4));
scientificScene.showWithId("protein", protein, select(protein).residues(), Style.cartoon().rainbowResidues());
scientificScene.recenterOn(molecule);
// @ts-expect-error Foreign source types must not collapse to any.
select({});
// @ts-expect-error Element filtering accepts the real CK Element type.
selectedAtoms.element(8);
// @ts-expect-error A rendering style is a native Style handle.
scientificScene.show(molecule, selectedAtoms, {kind: "sphere"});
void [shown, selectedIds];

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
const logged: void = handle.cameraParameterLogging(true);
const fps: void = handle.showFps(true);
const cameraOverlay: void = handle.showCameraParameters(true);
// @ts-expect-error Logging was renamed; no compatibility alias is exported.
handle.setCameraParameterLogging(true);
const screenshot: Promise<Uint8Array> = handle.takeScreenshot();
// @ts-expect-error Async output must be concrete, never Promise<any>.
const invalidScreenshot: Promise<string> = handle.takeScreenshot();
// @ts-expect-error RGB byte vectors are distinct from coordinate vectors.
scene.setBackgroundColor(new Float32Array([1, 0, 0]));
// @ts-expect-error Loop counts retain their bigint representation.
animation.setLoops(2);
const contract: string = bindingContractJson();
// Internal notebook paths remain separate from the public typed API.
const notebook: Promise<Viewer> = Viewer.renderNotebook("canvas-id", "scene-payload");
const submitted: void = handle.dispatch("command-payload");
// @ts-expect-error Notebook return-value endpoints are not supported.
handle.takeScreenshotNotebook();
// @ts-expect-error Native lifecycle methods must not appear in browser declarations.
handle.keepAlive();
// @ts-expect-error Normal JS render does not accept notebook transport payloads.
Viewer.render("compressed-scene", 800, 600);
// @ts-expect-error Normal JS update takes a Scene, not a transport payload.
handle.update("compressed-scene");
// @ts-expect-error Animation frames must be typed scenes.
animation.addFrame("compressed-scene");
void [playing, mounted, updated, logged, fps, cameraOverlay, screenshot, contract, notebook, submitted];
