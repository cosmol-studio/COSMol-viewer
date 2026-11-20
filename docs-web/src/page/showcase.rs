use dioxus::prelude::*;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use dioxus::web::WebEventExt;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct DockingOptions {
    pub pose: usize,
    pub overlay: bool,
    pub overview: bool,
    pub rotating: bool,
}

impl Default for DockingOptions {
    fn default() -> Self {
        Self {
            pose: 0,
            overlay: true,
            overview: false,
            rotating: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Example {
    Hero,
    Docking(DockingOptions),
    Conformer {
        series: usize,
        playing: bool,
        smooth: bool,
    },
    Interface {
        focused: bool,
        surface: bool,
    },
    Alignment(usize),
}

impl Example {
    fn name(self) -> &'static str {
        match self {
            Self::Hero => "hero",
            Self::Docking(_) => "docking",
            Self::Conformer { .. } => "conformer",
            Self::Interface { .. } => "interface",
            Self::Alignment(_) => "alignment",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewerStatus {
    Pending,
    Ready,
    Failed,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct ShowcaseViewer {
    status: Signal<ViewerStatus>,
    pub frame: Signal<usize>,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    active: Signal<Option<crate::gallery::Viewer>>,
}

impl ShowcaseViewer {
    pub(super) fn ready(self) -> bool {
        (self.status)() == ViewerStatus::Ready
    }

    pub(super) fn seek(self, _frame: usize) {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        if let Some(viewer) = self.active.peek().as_ref() {
            viewer.seek(_frame);
        }
    }
}

pub(super) fn use_viewer(_settings: impl Fn() -> Example + 'static) -> ShowcaseViewer {
    let viewer = ShowcaseViewer {
        status: use_signal(|| ViewerStatus::Pending),
        frame: use_signal(|| 1),
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        active: use_signal(|| None),
    };

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
        let mut active = viewer.active;
        use_effect(move || {
            let settings = _settings();
            if let Some(viewer_state) = active.write().as_mut()
                && let Err(error) = viewer_state.configure(settings)
            {
                crate::gallery::set_failed(viewer.status, error);
            }
        });
    }

    viewer
}

#[component]
pub(super) fn ViewerCanvas(
    viewer: ShowcaseViewer,
    example: Example,
    label: &'static str,
    loading: &'static str,
) -> Element {
    let name = example.name();
    let status = (viewer.status)();
    let message = if status == ViewerStatus::Failed {
        "Could not display this example. Reload to retry."
    } else {
        loading
    };
    rsx! {
        canvas {
            id: "{name}-canvas",
            tabindex: "0",
            aria_label: label,
            "data-live": (status == ViewerStatus::Ready).then_some("true"),
            onmounted: move |_event: MountedEvent| {
                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                {
                    let Some(canvas) = _event.data().try_as_web_event() else { return };
                    let mut viewer = viewer;
                    spawn(async move {
                        match crate::gallery::Viewer::start(canvas, example, viewer.frame, viewer.status).await {
                            Ok(active) => {
                                viewer.active.set(Some(active));
                                viewer.status.set(ViewerStatus::Ready);
                            }
                            Err(error) => crate::gallery::set_failed(viewer.status, error),
                        }
                    });
                }
            },
        }
        p { class: "canvas-loading", id: "{name}-loading", role: "status", hidden: status == ViewerStatus::Ready, "{message}" }
    }
}

#[component]
pub(super) fn FullscreenButton(id: &'static str, label: &'static str, disabled: bool) -> Element {
    let fullscreen = use_signal(|| false);
    let active = fullscreen();
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let mut panel = use_signal(|| None::<crate::gallery::FullscreenPanel>);

    rsx! {
        button {
            id,
            disabled,
            aria_label: if active { "Exit fullscreen" } else { label },
            onmounted: move |_event: MountedEvent| {
                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                if let Some(button) = _event.data().try_as_web_event() {
                    panel.set(crate::gallery::FullscreenPanel::new(button, fullscreen));
                }
            },
            onclick: move |_| {
                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                if let Some(panel) = panel.peek().as_ref() {
                    panel.toggle();
                }
            },
            if active { "Exit fullscreen" } else { "⛶" }
        }
    }
}
