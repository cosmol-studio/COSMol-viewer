use cosmol_viewer::{
    App, BallAndStick, Cartoon, Request, Scene, Surface,
    cosmolkit::{Element, Molecule, Protein, SdfReadParams, SdfRecordStream},
    eframe::{self, WebOptions, WebRunner, egui},
    select,
    styles::Stick,
    transport::{ViewerTransport, pair},
    utils::{Interpolatable, RustLogger},
};
use dioxus::prelude::{Signal, WritableExt};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
    time::Duration,
};
use wasm_bindgen::{JsCast, prelude::*};

use crate::page::showcase::{DockingOptions, Example, ViewerStatus};
use web_sys::{
    Element as DomElement, HtmlCanvasElement, HtmlInputElement, IntersectionObserver,
    IntersectionObserverEntry, IntersectionObserverInit,
};

const DOCKING_POSES: &str = include_str!("../data/showcase/imatinib-poses.sdf");
const SEQUENCES: [&str; 3] = [
    include_str!("../data/showcase/conformers-000.txt"),
    include_str!("../data/showcase/conformers-001.txt"),
    include_str!("../data/showcase/conformers-005.txt"),
];
const FRAME_SECONDS: f64 = 0.09;

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

fn error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn update(transport: &ViewerTransport, scene: Scene) -> Result<(), JsValue> {
    transport
        .send_request(Request::UpdateScene {
            request_id: 0,
            scene,
        })
        .map_err(error)
}

pub(crate) fn set_failed(mut status: Signal<ViewerStatus>, error: JsValue) {
    status.set(ViewerStatus::Failed);
    web_sys::console::warn_1(&error);
}

fn hero_scene() -> Result<Scene, JsValue> {
    let protein =
        Protein::from_pdb(include_str!("../data/showcase/1db1-receptor.pdb")).map_err(error)?;
    let ligand =
        Molecule::from_sdf(include_str!("../data/showcase/1db1-calcitriol.sdf")).map_err(error)?;
    let mut scene = Scene::new();
    scene.set_transparent_background(true);
    scene.set_auto_rotate(true, 8.0);
    scene.set_scale(0.23);
    scene.set_camera_view(-90.0, 0.0, -8.0, 35.0, [0.0; 3], 19.0);
    scene.recenter([14.8365, 20.829, 41.7925]);
    scene.set_ambient_light(0.47);
    scene.set_light_intensity(1.02);
    let residues = select(&protein).residues();
    let residue_count = protein.residues().count();
    let colors = ["#1748BA", "#239BD8", "#225CCA", "#35AFD5", "#2159C6"];
    for (index, color) in colors.iter().enumerate() {
        let start = index * residue_count / colors.len();
        let end = (index + 1) * residue_count / colors.len();
        let segment = residues
            .clone()
            .filter(|residue| (start..end).contains(&residue.index()));
        scene
            .add((&segment, Cartoon::default().color(*color).roughness(0.38)))
            .map_err(error)?;
    }
    scene.add((&ligand, Stick::default())).map_err(error)?;
    Ok(scene)
}

struct Docking {
    receptor: Protein,
    reference: Molecule,
    poses: Vec<Molecule>,
    receptor_center: [f32; 3],
    receptor_radius: f32,
}

impl Docking {
    fn load() -> Result<Self, JsValue> {
        let receptor =
            Protein::from_pdb(include_str!("../data/showcase/1iep-receptor.pdb")).map_err(error)?;
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for atom in receptor.atoms() {
            for (axis, value) in atom.position().into_iter().enumerate() {
                min[axis] = min[axis].min(value);
                max[axis] = max[axis].max(value);
            }
        }
        let center: [f64; 3] = std::array::from_fn(|i| (min[i] + max[i]) * 0.5);
        let radius = receptor
            .atoms()
            .map(|atom| {
                atom.position()
                    .into_iter()
                    .zip(center)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
            })
            .fold(0.0, f64::max)
            .sqrt();
        let mut molecules = SdfRecordStream::new(DOCKING_POSES.as_bytes())
            .map(|record| {
                let record = record.map_err(error)?;
                let molecule = record.molecule().map_err(error)?;
                molecule.with_kekulized_bonds().map_err(error)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if molecules.len() != 5 {
            return Err(error("Expected a crystal reference and four docking poses"));
        }
        let reference = molecules.remove(0);
        Ok(Self {
            receptor,
            reference,
            poses: molecules,
            receptor_center: center.map(|value| value as f32),
            receptor_radius: radius as f32,
        })
    }

    fn scene(&self, options: DockingOptions, canvas: &HtmlCanvasElement) -> Result<Scene, JsValue> {
        let receptor_color = "#36B5FA";
        let pose = &self.poses[options.pose];
        let mut scene = Scene::new();
        scene.set_transparent_background(true);
        scene.set_auto_rotate(options.rotating, 12.0);
        scene
            .add((&self.receptor, Cartoon::default().color(receptor_color)))
            .map_err(error)?;
        if options.overview {
            let canvas = canvas.get_bounding_client_rect();
            let aspect = (canvas.width() / canvas.height().max(1.0)).clamp(0.25, 1.0) as f32;
            let distance = 35.0;
            let fov = 15.0_f32;
            let half_angle = ((fov.to_radians() * 0.5).tan() * aspect).atan();
            scene.recenter(self.receptor_center);
            scene.set_scale(0.85 * distance * half_angle.sin() / (self.receptor_radius + 1.5));
            scene.set_camera_view(0.0, 0.0, 0.0, distance, [0.0; 3], fov);
        } else {
            scene.recenter_on(&self.reference).map_err(error)?;
            scene.set_scale(0.36);
            let coordinates = pose.coordinates_3d(0).map_err(error)?;
            let sidechain_atoms = self
                .receptor
                .residues()
                .filter(|residue| {
                    residue.atoms().any(|atom| {
                        let [x, y, z] = atom.position();
                        coordinates.iter().any(|point| {
                            (x - point[0]).powi(2) + (y - point[1]).powi(2) + (z - point[2]).powi(2)
                                <= 16.0
                        })
                    })
                })
                .flat_map(|residue| residue.atoms())
                // Keep Cα so each side chain can meet the cartoon backbone.
                .filter(|atom| {
                    atom.element() != Element::H
                        && !matches!(atom.name().as_str().trim(), "N" | "C" | "O" | "OXT")
                })
                .map(|atom| atom.id())
                .collect::<Vec<_>>();
            let sidechains = select(&self.receptor).atoms().filter_ids(&sidechain_atoms);
            if !sidechains.is_empty() {
                scene
                    .add((
                        &sidechains,
                        Stick::default().color(receptor_color).scale(0.65),
                    ))
                    .map_err(error)?;
            }
        }
        scene.add((pose, Stick::default())).map_err(error)?;
        if options.overlay {
            scene
                .add((
                    &self.reference,
                    Stick::default().color("#FF9E52").scale(0.67),
                ))
                .map_err(error)?;
        }
        Ok(scene)
    }
}

struct Timeline {
    molecule: Molecule,
    scene: Scene,
    sequences: [Vec<Vec<[f64; 3]>>; 3],
    frames: HashMap<(usize, usize), Scene>,
    series: usize,
    frame: usize,
    rendered: Option<(usize, usize)>,
    interpolated: bool,
    playing: bool,
    smooth: bool,
    next_frame: Option<f64>,
}

impl Timeline {
    fn load() -> Result<Self, JsValue> {
        let molecule = Molecule::from_sdf_with_params(
            include_str!("../data/showcase/conformer-topology.sdf"),
            &SdfReadParams {
                remove_hs: false,
                sanitize: false,
                ..Default::default()
            },
        )
        .map_err(error)?;
        let parse = |source: &str| -> Result<Vec<Vec<[f64; 3]>>, JsValue> {
            source
                .lines()
                .map(|line| {
                    let xyz = line
                        .split_ascii_whitespace()
                        .map(str::parse::<f64>)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(error)?;
                    if xyz.len() != molecule.num_atoms() * 3 || xyz.iter().any(|v| !v.is_finite()) {
                        return Err(error("Conformer frame does not match the shared topology"));
                    }
                    Ok(xyz.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect())
                })
                .collect()
        };
        let sequences = [
            parse(SEQUENCES[0])?,
            parse(SEQUENCES[1])?,
            parse(SEQUENCES[2])?,
        ];
        if sequences.iter().any(|frames| frames.len() != 50) {
            return Err(error("Expected 50 frames per generation sequence"));
        }
        let mut scene = Scene::new();
        scene.set_transparent_background(true);
        scene.set_scale(0.76);
        Ok(Self {
            molecule,
            scene,
            sequences,
            frames: HashMap::new(),
            series: 0,
            frame: 0,
            rendered: None,
            interpolated: false,
            playing: true,
            smooth: true,
            next_frame: None,
        })
    }

    fn frame_scene(&mut self, frame: usize) -> Result<Scene, JsValue> {
        let key = (self.series, frame);
        if let Some(scene) = self.frames.get(&key) {
            return Ok(scene.clone());
        }
        self.molecule
            .set_3d_coordinates_(self.sequences[self.series][frame].clone())
            .map_err(error)?;
        self.scene
            .set(
                "conformer",
                (
                    &self.molecule,
                    BallAndStick::default().silhouettes("#91ACC9", 1.0),
                ),
            )
            .map_err(error)?;
        self.scene.recenter_on(&self.molecule).map_err(error)?;
        self.frames.insert(key, self.scene.clone());
        Ok(self.scene.clone())
    }

    fn advance(
        &mut self,
        ui: &egui::Ui,
        transport: &ViewerTransport,
    ) -> Result<Option<usize>, JsValue> {
        let count = self.sequences[self.series].len();
        let mut fraction = 0.0;
        if self.playing {
            let now = ui.input(|state| state.time);
            let next = self.next_frame.get_or_insert(now + FRAME_SECONDS);
            if now >= *next {
                self.frame = (self.frame + 1) % count;
                *next = now + FRAME_SECONDS;
            }
            let remaining = (*next - now).max(0.0);
            if self.smooth && self.frame + 1 < count {
                fraction = (1.0 - remaining / FRAME_SECONDS).clamp(0.0, 1.0) as f32;
                ui.ctx().request_repaint();
            } else {
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64(remaining));
            }
        }
        let changed = self.rendered != Some((self.series, self.frame));
        if fraction > 0.0 || self.interpolated || changed {
            let current = self.frame_scene(self.frame)?;
            let scene = if fraction > 0.0 {
                current.interpolate(&self.frame_scene(self.frame + 1)?, fraction, RustLogger)
            } else {
                current
            };
            update(transport, scene)?;
            self.interpolated = fraction > 0.0;
        }
        if changed {
            self.rendered = Some((self.series, self.frame));
        }
        Ok(changed.then_some(self.frame + 1))
    }

    fn configure(&mut self, series: usize, playing: bool, smooth: bool) {
        if self.series != series {
            self.series = series;
            self.frame = 0;
            self.next_frame = None;
        }
        if self.playing != playing {
            self.playing = playing;
            self.next_frame = None;
        }
        self.smooth = smooth;
    }

    fn seek(&mut self, frame: usize) {
        self.playing = false;
        self.next_frame = None;
        self.frame = frame
            .saturating_sub(1)
            .min(self.sequences[self.series].len() - 1);
    }
}

fn interface_scenes() -> Result<[Scene; 2], JsValue> {
    const ANTIBODY: &str = "#78AAFF";
    const ANTIGEN: &str = "#FFB48E";
    let antibody =
        Protein::from_pdb(include_str!("../data/showcase/1vfb-antibody.pdb")).map_err(error)?;
    let antigen =
        Protein::from_pdb(include_str!("../data/showcase/1vfb-antigen.pdb")).map_err(error)?;
    let atoms = |protein: &Protein| {
        protein
            .atoms()
            .filter(|atom| atom.element().atomic_number() != 1)
            .map(|atom| (atom.residue().id(), atom.position()))
            .collect::<Vec<_>>()
    };
    let antibody_atoms = atoms(&antibody);
    let antigen_atoms = atoms(&antigen);
    let mut antibody_contacts = BTreeSet::new();
    let mut antigen_contacts = BTreeSet::new();
    let mut contact_points = Vec::new();
    for &(a_residue, a) in &antibody_atoms {
        for &(b_residue, b) in &antigen_atoms {
            if (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f64>() <= 16.0 {
                antibody_contacts.insert(a_residue);
                antigen_contacts.insert(b_residue);
                contact_points.push(std::array::from_fn(|i| (a[i] + b[i]) * 0.5));
            }
        }
    }
    if contact_points.is_empty() {
        return Err(error("Expected contacts between the antibody and antigen"));
    }

    let mut overview = Scene::new();
    overview.set_transparent_background(true);
    overview.set_scale(0.105);
    overview.rotate_camera(-20.0, -15.0, 10.0);
    let all_points = antibody_atoms.iter().chain(&antigen_atoms).map(|&(_, p)| p);
    overview.recenter(mean_point(all_points));
    overview
        .add((&antibody, Cartoon::default().color(ANTIBODY)))
        .map_err(error)?;
    overview
        .add((&antigen, Cartoon::default().color(ANTIGEN)))
        .map_err(error)?;
    let mut surface = Surface::default().color("#FFCEAD").opacity(0.43);
    surface.grid_spacing = 0.65;
    overview
        .set("antigen-surface", (&antigen, surface))
        .map_err(error)?;

    let mut focus = overview.clone();
    focus.recenter(mean_point(contact_points.into_iter()));
    focus.set_scale(0.24);
    for (protein, contacts, color) in [
        (&antibody, antibody_contacts, ANTIBODY),
        (&antigen, antigen_contacts, ANTIGEN),
    ] {
        let ids = contacts.into_iter().collect::<Vec<_>>();
        let sidechains = select(protein).atoms().residue_ids(&ids).sidechain();
        focus
            .add((&sidechains, Stick::default().color(color).scale(0.75)))
            .map_err(error)?;
    }

    Ok([overview, focus])
}

fn mean_point(points: impl Iterator<Item = [f64; 3]>) -> [f32; 3] {
    let (sum, count) = points.fold(([0.0; 3], 0), |(sum, count), p| {
        (std::array::from_fn(|axis| sum[axis] + p[axis]), count + 1)
    });
    sum.map(|value| (value / count as f64) as f32)
}

fn alignment_scenes() -> Result<[Scene; 3], JsValue> {
    let open = Protein::from_pdb(include_str!("../data/showcase/4ake-open.pdb")).map_err(error)?;
    let closed =
        Protein::from_pdb(include_str!("../data/showcase/1ake-aligned.pdb")).map_err(error)?;
    let scene = |show_open, show_closed| -> Result<Scene, JsValue> {
        let mut scene = Scene::new();
        scene.set_transparent_background(true);
        scene.set_scale(0.114);
        scene.rotate_camera(70.0, -20.0, 12.0);
        scene.recenter_on(&open).map_err(error)?;
        if show_open {
            scene
                .add((
                    &open,
                    Cartoon::default().color("#7AE9DC").opacity(if show_closed {
                        0.78
                    } else {
                        1.0
                    }),
                ))
                .map_err(error)?;
        }
        if show_closed {
            scene
                .add((&closed, Cartoon::default().color("#BB9DFF")))
                .map_err(error)?;
        }
        Ok(scene)
    };
    Ok([scene(true, true)?, scene(true, false)?, scene(false, true)?])
}

enum Scenes {
    Hero,
    Docking(Docking),
    Conformer(Rc<RefCell<Timeline>>),
    Interface([Scene; 2]),
    Alignment([Scene; 3]),
}

impl Scenes {
    fn scene(&self, example: Example, canvas: &HtmlCanvasElement) -> Result<Scene, JsValue> {
        match (self, example) {
            (Self::Hero, Example::Hero) => hero_scene(),
            (Self::Docking(docking), Example::Docking(options)) => docking.scene(options, canvas),
            (Self::Conformer(timeline), Example::Conformer { .. }) => {
                timeline.borrow_mut().frame_scene(0)
            }
            (Self::Interface(scenes), Example::Interface { focused, surface }) => {
                let mut scene = scenes[usize::from(focused)].clone();
                if !surface {
                    let _ = scene.remove_shape("antigen-surface");
                }
                Ok(scene)
            }
            (Self::Alignment(scenes), Example::Alignment(mode)) => Ok(scenes[mode].clone()),
            _ => unreachable!("a viewer is mounted for exactly one example"),
        }
    }
}

struct EmbeddedViewer {
    app: App<RustLogger>,
    canvas: HtmlCanvasElement,
    startup_scroll: Option<(f64, f64)>,
    timeline: Option<Rc<RefCell<Timeline>>>,
    frame_number: Signal<usize>,
    status: Signal<ViewerStatus>,
    transport: ViewerTransport,
}

impl eframe::App for EmbeddedViewer {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let window = web_sys::window().unwrap();
        if ui.input(|state| {
            state.pointer.any_pressed()
                || state
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Key { .. }))
        }) {
            self.startup_scroll = None;
        }
        if let Some((x, y)) = self.startup_scroll {
            if document().active_element().as_ref() == Some(self.canvas.as_ref()) {
                self.canvas.blur().ok();
                window.scroll_to_with_x_and_y(x, y);
                self.startup_scroll = None;
            }
        }

        let rect = self.canvas.get_bounding_client_rect();
        let height = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if document().hidden() || rect.bottom() <= 0.0 || rect.top() >= height {
            ui.ctx().request_repaint_after(Duration::from_millis(150));
            return;
        }
        if let Some(timeline) = &self.timeline {
            let mut timeline = timeline.borrow_mut();
            match timeline.advance(ui, &self.transport) {
                Ok(Some(frame_number)) => self.frame_number.set(frame_number),
                Ok(None) => {}
                Err(err) => {
                    timeline.playing = false;
                    timeline.rendered = Some((timeline.series, timeline.frame));
                    set_failed(self.status, err);
                }
            }
            // Also pick up slider and select changes while paused.
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        self.app.ui(ui, frame);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.app.on_exit(gl);
        self.transport.close();
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color(visuals)
    }
}

struct Runner(WebRunner);

impl Drop for Runner {
    fn drop(&mut self) {
        self.0.destroy();
    }
}

pub(crate) struct Viewer {
    _runner: Runner,
    canvas: HtmlCanvasElement,
    transport: ViewerTransport,
    scenes: Scenes,
    options: Example,
}

impl Viewer {
    pub(crate) async fn start(
        canvas: DomElement,
        example: Example,
        frame_number: Signal<usize>,
        status: Signal<ViewerStatus>,
    ) -> Result<Self, JsValue> {
        let canvas = canvas.dyn_into::<HtmlCanvasElement>()?;
        if example != Example::Hero {
            wait_until_visible(&canvas).await?;
        }
        let scenes = match example {
            Example::Hero => Scenes::Hero,
            Example::Docking(_) => Scenes::Docking(Docking::load()?),
            Example::Conformer { .. } => {
                Scenes::Conformer(Rc::new(RefCell::new(Timeline::load()?)))
            }
            Example::Interface { .. } => Scenes::Interface(interface_scenes()?),
            Example::Alignment(_) => Scenes::Alignment(alignment_scenes()?),
        };
        let scene = scenes.scene(example, &canvas)?;
        let timeline = match &scenes {
            Scenes::Conformer(timeline) => Some(timeline.clone()),
            _ => None,
        };
        let host_canvas = canvas.clone();
        let runner = Runner(WebRunner::new());
        let (transport, renderer) = pair();
        let app_transport = transport.clone();
        let window = web_sys::window().unwrap();
        let scroll = (window.scroll_x()?, window.scroll_y()?);
        runner
            .0
            .start(
                canvas.clone(),
                WebOptions::default(),
                Box::new(move |cc| {
                    let mut app = App::new(cc, &scene, RustLogger);
                    app.attach_transport(renderer);
                    Ok(Box::new(EmbeddedViewer {
                        app,
                        canvas: host_canvas,
                        startup_scroll: Some(scroll),
                        timeline,
                        frame_number,
                        status,
                        transport: app_transport,
                    }))
                }),
            )
            .await?;

        // eframe creates an autofocus input for IME. These viewers accept only pointer input.
        let inputs = document().query_selector_all("body > input[autofocus]")?;
        for index in 0..inputs.length() {
            if let Some(input) = inputs
                .item(index)
                .and_then(|node| node.dyn_into::<HtmlInputElement>().ok())
            {
                input.remove_attribute("autofocus")?;
                input.blur().ok();
            }
        }

        Ok(Self {
            _runner: runner,
            canvas,
            transport,
            scenes,
            options: example,
        })
    }

    pub(crate) fn configure(&mut self, example: Example) -> Result<(), JsValue> {
        if example == self.options {
            return Ok(());
        }
        if let (
            Scenes::Conformer(timeline),
            Example::Conformer {
                series,
                playing,
                smooth,
            },
        ) = (&self.scenes, example)
        {
            timeline.borrow_mut().configure(series, playing, smooth);
        } else {
            let scene = self.scenes.scene(example, &self.canvas)?;
            let reframe = matches!(
                (self.options, example),
                (Example::Docking(before), Example::Docking(after)) if before.overview != after.overview
            );
            if reframe {
                let bounds = self.canvas.get_bounding_client_rect();
                self.transport
                    .send_request(Request::InitializeScene {
                        request_id: 0,
                        scene,
                        width: bounds.width() as f32,
                        height: bounds.height() as f32,
                    })
                    .map_err(error)?;
            } else {
                update(&self.transport, scene)?;
            }
        }
        self.options = example;
        Ok(())
    }

    pub(crate) fn seek(&self, frame: usize) {
        if let Scenes::Conformer(timeline) = &self.scenes {
            timeline.borrow_mut().seek(frame);
        }
    }
}

struct VisibilityObserver(IntersectionObserver);

impl Drop for VisibilityObserver {
    fn drop(&mut self) {
        self.0.disconnect();
    }
}

async fn wait_until_visible(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let (ready, visible) = futures_channel::oneshot::channel();
    let mut ready = Some(ready);
    let callback = Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new(
        move |entries: js_sys::Array, observer: IntersectionObserver| {
            if entries.iter().any(|entry| {
                entry
                    .unchecked_into::<IntersectionObserverEntry>()
                    .is_intersecting()
            }) {
                observer.disconnect();
                if let Some(ready) = ready.take() {
                    let _ = ready.send(());
                }
            }
        },
    );
    let options = IntersectionObserverInit::new();
    options.set_root_margin("120px 0px");
    let observer = VisibilityObserver(IntersectionObserver::new_with_options(
        callback.as_ref().unchecked_ref(),
        &options,
    )?);
    observer.0.observe(canvas);
    visible.await.map_err(error)
}

pub(crate) struct FullscreenPanel {
    panel: DomElement,
    changed: Closure<dyn FnMut()>,
}

impl FullscreenPanel {
    pub(crate) fn new(button: DomElement, mut active: Signal<bool>) -> Option<Self> {
        let panel = button.closest(".live-panel").ok().flatten()?;
        let target = panel.clone();
        let changed = Closure::<dyn FnMut()>::new(move || {
            active.set(document().fullscreen_element().as_ref() == Some(&target));
        });
        panel
            .add_event_listener_with_callback("fullscreenchange", changed.as_ref().unchecked_ref())
            .ok()?;
        Some(Self { panel, changed })
    }

    pub(crate) fn toggle(&self) {
        if document().fullscreen_element().as_ref() == Some(&self.panel) {
            document().exit_fullscreen();
        } else {
            let _ = self.panel.request_fullscreen();
        }
    }
}

impl Drop for FullscreenPanel {
    fn drop(&mut self) {
        let _ = self.panel.remove_event_listener_with_callback(
            "fullscreenchange",
            self.changed.as_ref().unchecked_ref(),
        );
    }
}
