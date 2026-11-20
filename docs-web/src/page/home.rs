use dioxus::prelude::*;

use super::showcase::{DockingOptions, Example, FullscreenButton, ViewerCanvas, use_viewer};
use crate::{component::Seo, route::Route};

const WEBSITE_JSON_LD: &str = r#"{
  "@context": "https://schema.org",
  "@type": "WebSite",
  "name": "COSMol Viewer",
  "alternateName": "COSMol Viewer Documentation",
  "url": "https://viewer.cosmol.org/"
}"#;

#[component]
pub fn Home() -> Element {
    rsx! {
        Seo {
            title: "COSMol Viewer documentation — Guides and API reference",
            description: "Learn COSMol Viewer with Python and JavaScript guides, API references, and interactive molecular visualization examples.",
        }
        document::Script { r#type: "application/ld+json", "{WEBSITE_JSON_LD}" }
        document::Link { rel: "stylesheet", href: asset!("/assets/gallery.css") }
        div { class: "docs-home",
            main { role: "main", class: "docs-home-inner",
                section { class: "docs-intro docs-intro-live", id: "gallery-stage", aria_label: "Documentation overview",
                    div { class: "docs-intro-copy",
                        p { class: "home-kicker", "DOCUMENTATION" }
                        h1 { "COSMol Viewer" }
                        p { class: "docs-intro-description", "A Rust-native library for molecular visualization. Build interactive 3D views of molecules and proteins, animate structural changes, and export figures from Python, JavaScript / TypeScript, or Rust." }
                        div { class: "docs-intro-actions",
                            Link { class: "docs-primary-action", to: Route::Quickstart { fragment: String::new() }, "Python quick start", span { aria_hidden: "true", "→" } }
                            Link { class: "docs-primary-action", to: Route::JavaScriptQuickstart { fragment: String::new() }, "JavaScript quick start", span { aria_hidden: "true", "→" } }
                            Link { class: "docs-text-action", to: Route::Api { fragment: String::new() }, "Python API reference", span { aria_hidden: "true", "→" } }
                            Link { class: "docs-text-action", to: Route::JavaScriptApi { fragment: String::new() }, "JavaScript API reference", span { aria_hidden: "true", "→" } }
                        }
                    }
                    HeroStructure {}
                }
                Applications {}
                section { class: "docs-guide-section", aria_label: "Explore the documentation",
                    div { class: "docs-section-heading",
                        h2 { "Guides and API reference" }
                        div { class: "flex flex-wrap gap-x-5 gap-y-2",
                            Link { class: "docs-text-action", to: Route::Python {}, "All Python documentation →" }
                            Link { class: "docs-text-action", to: Route::JavaScript { fragment: String::new() }, "All JavaScript documentation →" }
                        }
                    }
                    div { class: "docs-topic-grid home-guide-grid",
                        TopicGroup { title: "Getting started", description: "Set up COSMol Viewer and render your first scene.",
                            TopicLink { title: "Install for Python", to: Route::Installation { fragment: String::new() } }
                            TopicLink { title: "Python quick start", to: Route::Quickstart { fragment: String::new() } }
                            TopicLink { title: "JavaScript quick start", to: Route::JavaScriptQuickstart { fragment: String::new() } }
                        }
                        TopicGroup { title: "Scenes & geometry", description: "Compose molecules, proteins, and annotation geometry.",
                            TopicLink { title: "Molecule representations", to: Route::Molecules { fragment: String::new() } }
                            TopicLink { title: "Scenes and cameras", to: Route::Scenes { fragment: String::new() } }
                            TopicLink { title: "Spheres and sticks", to: Route::Geometry { fragment: String::new() } }
                        }
                        TopicGroup { title: "Rendering workflows", description: "Control output, interaction, and molecular animations.",
                            TopicLink { title: "PNG export and notebooks", to: Route::Rendering { fragment: String::new() } }
                            TopicLink { title: "Viewers and animation", to: Route::Viewer { fragment: String::new() } }
                            TopicLink { title: "Protein structures", to: Route::Proteins { fragment: String::new() } }
                        }
                        TopicGroup { title: "Look up the details", description: "Check signatures and locate symbols.",
                            TopicLink { title: "Search documentation", to: Route::SearchPage { q: String::new(), fragment: String::new() } }
                            TopicLink { title: "Python API reference", to: Route::Api { fragment: String::new() } }
                            TopicLink { title: "JavaScript API reference", to: Route::JavaScriptApi { fragment: String::new() } }
                            TopicLink { title: "General index", to: Route::Genindex { fragment: String::new() } }
                        }
                    }
                }
                section { class: "docs-related", aria_label: "Related COSMol Viewer websites",
                    div { class: "docs-section-heading",
                        h2 { "COSMol ecosystem" }
                    }
                    div { class: "docs-related-grid home-ecosystem-grid",
                        a { class: "docs-external-card docs-external-kit", href: "https://kit.cosmol.org/", target: "_blank", rel: "noreferrer",
                            h3 { "COSMolKit" }
                            p { "Load molecules and proteins, generate conformers, and pass the resulting structures directly to Viewer." }
                            div { class: "docs-external-destination", span { "Explore COSMolKit" } span { "kit.cosmol.org ↗" } }
                        }
                        a { class: "docs-external-card docs-external-tools", href: "https://tools.cosmol.org/tools", target: "_blank", rel: "noreferrer",
                            h3 { "COSMolKit Web tools" }
                            p { "Convert molecular formats, render structures, and explore chemistry directly in your browser. A separate workspace for hands-on tasks." }
                            div { class: "docs-external-destination", span { "Open Web tools" } span { "tools.cosmol.org/tools ↗" } }
                        }
                        a { class: "docs-external-card docs-external-blog", href: "https://tools.cosmol.org/blog", target: "_blank", rel: "noreferrer",
                            h3 { "From the blog" }
                            p { "Read beyond the reference: articles about cheminformatics, the toolkit, and the ideas behind the project." }
                            div { class: "docs-external-destination", span { "Read the blog" } span { "tools.cosmol.org/blog ↗" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn HeroStructure() -> Element {
    let viewer = use_viewer(|| Example::Hero);
    rsx! {
        div { class: "hero-demo",
            div { class: "live-panel hero-structure", id: "hero-panel",
                div { class: "stage-viewport",
                    ViewerCanvas {
                        viewer,
                        example: Example::Hero,
                        label: "Human vitamin D receptor bound to calcitriol. Drag to rotate and scroll to zoom.",
                        loading: "Loading molecular structure…",
                    }
                }
            }
        }
    }
}

#[component]
fn DockingDemo() -> Element {
    let mut options = use_signal(DockingOptions::default);
    let viewer = use_viewer(move || Example::Docking(options()));
    let disabled = !viewer.ready();
    let settings = options();
    rsx! {
        article { class: "application-card", id: "docking-demo",
            h3 { "Review molecular docking results" }
            p { "Compare predicted imatinib poses with its crystal pose to see which parts of the ligand agree." }
            div { class: "live-panel", id: "docking-panel",
                div { class: "stage-viewport",
                    ViewerCanvas {
                        viewer,
                        example: Example::Docking(settings),
                        label: "Imatinib in the c-Abl binding site. Drag to rotate and scroll to zoom.",
                        loading: "Loading the docking results…",
                    }
                    div { class: "stage-legend", aria_label: "Pose styles",
                        span { class: "legend-prediction", "Predicted pose" }
                        span { class: "legend-crystal", id: "crystal-legend", hidden: !settings.overlay, "Crystal reference" }
                    }
                }
                div { class: "stage-toolbar",
                    label { r#for: "docking-pose", "Predicted pose"
                        select {
                            id: "docking-pose",
                            disabled,
                            value: "{settings.pose}",
                            onchange: move |event| {
                                if let Ok(pose @ 0..=3) = event.value().parse::<usize>() {
                                    options.write().pose = pose;
                                }
                            },
                            option { value: "0", "1 · −13.234 kcal/mol" }
                            option { value: "1", "2 · −11.293 kcal/mol" }
                            option { value: "2", "3 · −11.281 kcal/mol" }
                            option { value: "3", "4 · −11.146 kcal/mol" }
                        }
                    }
                    label { class: "check-control",
                        input {
                            id: "docking-reference", r#type: "checkbox", checked: settings.overlay, disabled,
                            onchange: move |event| options.write().overlay = event.checked(),
                        }
                        "Crystal overlay"
                    }
                    button {
                        id: "docking-view", disabled,
                        onclick: move |_| { let mut options = options.write(); options.overview = !options.overview; },
                        if settings.overview { "Binding site" } else { "Full receptor" }
                    }
                    button {
                        id: "docking-rotate", disabled,
                        onclick: move |_| { let mut options = options.write(); options.rotating = !options.rotating; },
                        if settings.rotating { "Pause rotation" } else { "Resume rotation" }
                    }
                    FullscreenButton { id: "docking-fullscreen", label: "Fullscreen docking viewer", disabled }
                }
            }
        }
    }
}

#[component]
fn Applications() -> Element {
    rsx! {
        section { class: "docs-guide-section home-applications", id: "applications", aria_label: "Application examples",
            div { class: "docs-section-heading",
                h2 { "What you can build" }
            }
            div { class: "application-grid",
                DockingDemo {}
                ConformerDemo {}
                InterfaceDemo {}
                AlignmentDemo {}
            }
        }
    }
}

#[component]
fn ConformerDemo() -> Element {
    let mut series = use_signal(|| 0);
    let mut playing = use_signal(|| true);
    let mut smooth = use_signal(|| true);
    let options = move || Example::Conformer {
        series: series(),
        playing: playing(),
        smooth: smooth(),
    };
    let viewer = use_viewer(options);
    let disabled = !viewer.ready();
    let frame = (viewer.frame)();
    rsx! {
        article { class: "application-card", id: "conformer-generation",
            h3 { "Inspect conformer generation" }
            p { "Play a generation sequence or scrub through its frames to see how a molecule changes shape." }
            div { class: "live-panel", id: "conformer-panel",
                div { class: "stage-viewport",
                    ViewerCanvas {
                        viewer,
                        example: options(),
                        label: "Molecular conformer playback. Drag to rotate and scroll to zoom.",
                        loading: "Preparing conformer frames…",
                    }
                    span { class: "frame-label", id: "conformer-readout", "Frame {frame} / 50" }
                }
                div { class: "stage-toolbar timeline",
                    button {
                        id: "conformer-play", disabled,
                        onclick: move |_| playing.toggle(),
                        if playing() { "Pause" } else { "Play" }
                    }
                    input {
                        id: "conformer-frame", r#type: "range", min: "1", max: "50",
                        value: "{frame}", disabled, aria_label: "Conformer frame",
                        oninput: move |event| {
                            if let Ok(frame) = event.value().parse() {
                                playing.set(false);
                                viewer.seek(frame);
                            }
                        },
                    }
                    label {
                        input {
                            id: "conformer-smooth", r#type: "checkbox", checked: smooth(), disabled,
                            onchange: move |event| smooth.set(event.checked()),
                        }
                        "Smooth"
                    }
                    label { r#for: "conformer-series", "Sequence"
                        select {
                            id: "conformer-series", disabled, value: "{series}",
                            onchange: move |event| {
                                if let Ok(index @ 0..=2) = event.value().parse::<usize>() {
                                    series.set(index);
                                }
                            },
                            option { value: "0", "000" }
                            option { value: "1", "001" }
                            option { value: "2", "005" }
                        }
                    }
                    FullscreenButton { id: "conformer-fullscreen", label: "Fullscreen conformer viewer", disabled }
                }
            }
        }
    }
}

#[component]
fn InterfaceDemo() -> Element {
    let mut focused = use_signal(|| false);
    let mut surface = use_signal(|| true);
    let options = move || Example::Interface {
        focused: focused(),
        surface: surface(),
    };
    let viewer = use_viewer(options);
    let disabled = !viewer.ready();
    rsx! {
        article { class: "application-card", id: "antibody-interface",
            h3 { "Inspect an antibody–antigen interface" }
            p { "See how antibody D1.3 fits against the lysozyme surface, then zoom in to inspect the contact residues." }
            div { class: "live-panel", id: "interface-panel",
                div { class: "stage-viewport",
                    ViewerCanvas {
                        viewer,
                        example: options(),
                        label: "Antibody D1.3 and lysozyme. Drag to rotate and scroll to zoom.",
                        loading: "Loading the complex…",
                    }
                    div { class: "stage-legend", aria_label: "Structure colors",
                        span { class: "legend-antibody", "Antibody D1.3" }
                        span { class: "legend-antigen", "Lysozyme" }
                    }
                }
                div { class: "stage-toolbar",
                    button {
                        id: "interface-focus", disabled,
                        onclick: move |_| focused.toggle(),
                        if focused() { "Whole complex" } else { "Focus interface" }
                    }
                    label {
                        input {
                            id: "interface-surface", r#type: "checkbox", checked: surface(), disabled,
                            onchange: move |event| surface.set(event.checked()),
                        }
                        "Molecular surface"
                    }
                    FullscreenButton { id: "interface-fullscreen", label: "Fullscreen interface viewer", disabled }
                }
            }
        }
    }
}

#[component]
fn AlignmentDemo() -> Element {
    let mut mode = use_signal(|| 0);
    let options = move || Example::Alignment(mode());
    let viewer = use_viewer(options);
    let disabled = !viewer.ready();
    rsx! {
        article { class: "application-card", id: "structure-comparison",
            h3 { "Compare open and closed protein states" }
            p { "Overlay two aligned structures of adenylate kinase to see which parts move as the enzyme closes." }
            div { class: "live-panel", id: "alignment-panel",
                div { class: "stage-viewport",
                    ViewerCanvas {
                        viewer,
                        example: options(),
                        label: "Aligned open and closed adenylate kinase structures. Drag to rotate and scroll to zoom.",
                        loading: "Loading aligned structures…",
                    }
                    div { class: "stage-legend", aria_label: "Structure colors",
                        span { class: "legend-open", id: "alignment-open-legend", hidden: mode() == 2, "Open" }
                        span { class: "legend-closed", id: "alignment-closed-legend", hidden: mode() == 1, "Closed" }
                    }
                }
                div { class: "stage-toolbar",
                    label { r#for: "alignment-mode", "Display"
                        select {
                            id: "alignment-mode", disabled, value: "{mode}",
                            onchange: move |event| {
                                if let Ok(index @ 0..=2) = event.value().parse::<usize>() {
                                    mode.set(index);
                                }
                            },
                            option { value: "0", "Overlay" }
                            option { value: "1", "Open only" }
                            option { value: "2", "Closed only" }
                        }
                    }
                    FullscreenButton { id: "alignment-fullscreen", label: "Fullscreen structure comparison", disabled }
                }
            }
        }
    }
}

#[component]
fn TopicGroup(title: &'static str, description: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "docs-topic-group",
            h3 { "{title}" }
            p { "{description}" }
            div { class: "docs-topic-links", {children} }
        }
    }
}

#[component]
fn TopicLink(title: &'static str, to: Route) -> Element {
    rsx! {
        Link { class: "docs-topic-link", to, "{title}", span { aria_hidden: "true", "→" } }
    }
}
