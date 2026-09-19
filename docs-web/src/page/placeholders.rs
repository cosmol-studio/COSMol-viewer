use dioxus::prelude::*;

use crate::{component::Seo, route::Route};

#[component]
pub fn JavaScript() -> Element {
    rsx! { Placeholder { eyebrow: "JAVASCRIPT API", title: "JavaScript documentation is reserved", body: "The standalone JavaScript scene-building API is planned. Python notebook canvases already use the WebAssembly renderer; they do not require a separate JavaScript installation." } }
}

#[component]
pub fn Benchmarks() -> Element {
    rsx! { Placeholder { eyebrow: "BENCHMARKS", title: "Benchmark reports are reserved", body: "Reproducible benchmark reports will be published here as browser and native measurements are promoted into the maintained documentation surface." } }
}

#[component]
pub fn Validation() -> Element {
    rsx! {
        Seo { title: "COSMol Viewer Validation | Rendering and build checks", description: "Review COSMol Viewer rendering checks, examples, documentation tests, and the limits of cross-platform image comparisons." }
        div { class: "min-h-screen uu-backdrop m-0 pt-[74px]",
            main { role: "main", class: "mx-auto w-full max-w-4xl px-6 py-10 font-sans text-[#e8edf5] max-[640px]:px-3.5 max-[640px]:py-7",
                span { class: "text-xs font-bold tracking-[0.08em] text-[#4b96ff]", "VALIDATION" }
                h1 { class: "mb-4 mt-3 text-[28px] leading-[1.35] font-bold text-white", "Rendering and compatibility checks" }
                p { class: "max-w-[760px] text-[16px] leading-7 text-[#aebacd]", "The shared Rust renderer serves native windows, notebook canvases, and static image export. Source tests and runnable examples cover molecular geometry, protein surfaces, camera state, and rendering settings." }
                div { class: "mt-8 grid grid-cols-3 gap-4 max-[760px]:grid-cols-1",
                    Stat { value: "Rust core", label: "Geometry and state tests" }
                    Stat { value: "Python 0.3.0", label: "Documentation build package" }
                    Stat { value: "Browser + native", label: "Rendering environments" }
                }
                section { class: "mt-10 border-t border-white/10 pt-8",
                    h2 { class: "text-xl font-bold text-white", "Inspect the checks and examples" }
                    p { class: "mt-3 text-[14px] leading-6 text-[#9caabd]", "The repository contains core unit tests and Python rendering examples. Driver, antialiasing, and platform differences can affect pixels; this documentation does not claim exact image parity across all backends. The documentation build validates routes, metadata, links, and browser search separately." }
                    a { class: "mt-5 inline-flex rounded-md bg-[#3082ff] px-4 py-2.5 text-sm font-bold text-white no-underline hover:bg-[#438ee9]", href: "https://github.com/cosmol-studio/COSMol-viewer/tree/main/crates/core/src", target: "_blank", rel: "noreferrer", "Browse renderer source and tests" }
                }
            }
        }
    }
}

#[component]
fn Placeholder(eyebrow: &'static str, title: &'static str, body: &'static str) -> Element {
    rsx! {
        Seo { title: "{title} | COSMol Viewer", description: "{body}" }
        div { class: "min-h-screen uu-backdrop m-0 pt-[74px]",
            main { role: "main", class: "mx-auto w-full max-w-4xl px-6 py-10 font-sans text-[#e8edf5] max-[640px]:px-3.5 max-[640px]:py-7",
                Link { class: "text-[13px] font-semibold text-[#7ab5ff] no-underline hover:text-[#b4d6ff]", to: Route::Home {}, "Back to documentation" }
                span { class: "mt-8 block text-xs font-bold tracking-[0.08em] text-[#4b96ff]", "{eyebrow}" }
                h1 { class: "mb-4 mt-3 text-[28px] leading-[1.35] font-bold text-white", "{title}" }
                p { class: "max-w-[680px] text-[16px] leading-7 text-[#aebacd]", "{body}" }
            }
        }
    }
}

#[component]
fn Stat(value: &'static str, label: &'static str) -> Element {
    rsx! { div { class: "rounded-md border border-[#28415f] bg-[#0b1727] p-4", strong { class: "block text-sm font-bold text-white", "{value}" }, span { class: "mt-1 block text-xs text-[#8495aa]", "{label}" } } }
}
