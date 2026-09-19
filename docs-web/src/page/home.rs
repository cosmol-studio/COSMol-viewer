use dioxus::prelude::*;

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
            description: "Learn COSMol Viewer with installation instructions, Python guides, API reference, and rendering and export workflows.",

        }
        document::Script { r#type: "application/ld+json", "{WEBSITE_JSON_LD}" }
        div { class: "docs-home",
            main { role: "main", class: "docs-home-inner",
                section { class: "docs-intro", aria_label: "Documentation overview",
                    div {
                        p { class: "home-kicker", "THE COSMOL VIEWER HANDBOOK" }
                        h1 { "Documentation" }
                        p { class: "docs-intro-description", "A practical guide to the Rust-native molecular viewer. Learn the Python API, compose molecular scenes, and find the details you need." }
                        div { class: "docs-intro-actions",
                            Link { class: "docs-primary-action", to: Route::Quickstart { fragment: String::new() }, "Start the quick guide", span { aria_hidden: "true", "→" } }
                            Link { class: "docs-text-action", to: Route::Api { fragment: String::new() }, "Browse API reference", span { aria_hidden: "true", "→" } }
                        }
                    }
                    aside { class: "docs-reading-path", aria_label: "Getting started",
                        p { class: "home-kicker", "NEW TO COSMOL VIEWER?" }
                        StartLink { number: "01", title: "Install the package", detail: "Set up your Python environment", to: Route::Installation { fragment: String::new() } }
                        StartLink { number: "02", title: "Render your first molecule", detail: "Follow the quick start", to: Route::Quickstart { fragment: String::new() } }
                        StartLink { number: "03", title: "Compose a scene", detail: "Learn the core data model", to: Route::Scenes { fragment: String::new() } }
                    }
                }
                section { class: "docs-guide-section", aria_label: "Explore the documentation",
                    div { class: "docs-section-heading",
                        div { p { class: "home-kicker", "GUIDES & REFERENCE" } h2 { "Find your next step" } }
                        Link { class: "docs-text-action", to: Route::Python {}, "All Python documentation →" }
                    }
                    div { class: "docs-topic-grid",
                        TopicGroup { number: "01", title: "Scenes & geometry", description: "Compose molecules, proteins, and annotation geometry.",
                            TopicLink { title: "Molecule representations", to: Route::Molecules { fragment: String::new() } }
                            TopicLink { title: "Scenes and cameras", to: Route::Scenes { fragment: String::new() } }
                            TopicLink { title: "Spheres and sticks", to: Route::Geometry { fragment: String::new() } }
                        }
                        TopicGroup { number: "02", title: "Rendering workflows", description: "Control output, interaction, and molecular animations.",
                            TopicLink { title: "PNG export and notebooks", to: Route::Rendering { fragment: String::new() } }
                            TopicLink { title: "Viewers and animation", to: Route::Viewer { fragment: String::new() } }
                            TopicLink { title: "Rust API", to: Route::Rust { fragment: String::new() } }
                            TopicLink { title: "Protein structures", to: Route::Proteins { fragment: String::new() } }
                        }
                        TopicGroup { number: "03", title: "Look up the details", description: "Check signatures, locate symbols, and review evidence.",
                            TopicLink { title: "Search documentation", to: Route::SearchPage { q: String::new(), fragment: String::new() } }
                            TopicLink { title: "Python API reference", to: Route::Api { fragment: String::new() } }
                            TopicLink { title: "General index", to: Route::Genindex { fragment: String::new() } }
                            TopicLink { title: "Validation evidence", to: Route::Validation {} }
                        }
                    }
                    div { class: "docs-availability",
                        span { class: "docs-status-label", "NOT YET AVAILABLE" }
                        Link { to: Route::JavaScript {}, "JavaScript / WebAssembly documentation" }
                        span { aria_hidden: "true", "·" }
                        Link { to: Route::Benchmarks {}, "Benchmark reports" }
                    }
                }
                section { class: "docs-related", aria_label: "Related COSMol Viewer websites",
                    div { class: "docs-section-heading",
                        div { p { class: "home-kicker", "BEYOND THE DOCUMENTATION" } h2 { "Explore the wider project" } }
                        p { class: "docs-related-note", "Companion resources on tools.cosmol.org" }
                    }
                    div { class: "docs-related-grid",
                        a { class: "docs-external-card docs-external-tools", href: "https://tools.cosmol.org/tools", target: "_blank", rel: "noreferrer",
                            div { class: "docs-external-meta", span { "INTERACTIVE WORKSPACE" } span { "EXTERNAL ↗" } }
                            h3 { "COSMol Web tools" }
                            p { "Convert molecular formats, render structures, and explore chemistry directly in your browser. A separate workspace for hands-on tasks." }
                            div { class: "docs-external-destination", span { "Open Web tools" } span { "tools.cosmol.org/tools ↗" } }
                        }
                        a { class: "docs-external-card docs-external-blog", href: "https://tools.cosmol.org/blog", target: "_blank", rel: "noreferrer",
                            div { class: "docs-external-meta", span { "ARTICLES & PERSPECTIVES" } span { "EXTERNAL ↗" } }
                            h3 { "From the blog" }
                            p { "Read beyond the reference: articles about cheminformatics, the toolkit, and the ideas behind the project." }
                            div { class: "docs-external-destination", span { "Read the blog" } span { "tools.cosmol.org/blog ↗" } }
                        }
                    }
                    p { class: "docs-external-disclosure", "These links open the companion website in a new tab. Guides and API reference stay here." }
                }
            }
        }
    }
}

#[component]
fn StartLink(
    number: &'static str,
    title: &'static str,
    detail: &'static str,
    to: Route,
) -> Element {
    rsx! {
        Link { class: "docs-start-link", to,
            span { class: "docs-step-number", "{number}" }
            div { strong { "{title}" } span { "{detail}" } }
            span { class: "docs-link-arrow", aria_hidden: "true", "→" }
        }
    }
}

#[component]
fn TopicGroup(
    number: &'static str,
    title: &'static str,
    description: &'static str,
    children: Element,
) -> Element {
    rsx! {
        div { class: "docs-topic-group",
            span { class: "docs-topic-number", "{number}" }
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
