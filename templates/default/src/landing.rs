//! The site root. Docs live under `base_path`, which leaves `/` free for this page.

use dioxus::prelude::*;

use crate::docs::{pages, SITE_CONFIG};
use crate::theme::{DocLink, SiteFooter, SiteHeader, ThemeStyles, Wordmark};

#[component]
pub fn Landing() -> Element {
    rsx! {
        ThemeStyles {}
        document::Title { "{SITE_CONFIG.title}" }

        div { class: "flex min-h-screen flex-col",
            SiteHeader { config: SITE_CONFIG, pages: pages() }

            main { class: "mx-auto w-full max-w-3xl flex-1 px-5 pt-20 pb-16",
                section { class: "text-center",
                    h1 { class: "m-0 mb-6",
                        Wordmark {
                            text: SITE_CONFIG.title.to_string(),
                            class: "text-[clamp(2.4rem,6vw,3.6rem)] leading-none",
                        }
                    }
                    if let Some(description) = SITE_CONFIG.description {
                        p { class: "mx-auto mb-8 max-w-xl text-lg text-muted", "{description}" }
                    }
                    div { class: "flex flex-wrap items-center justify-center gap-3",
                        DocLink {
                            route: SITE_CONFIG.docs_root.to_string(),
                            class: "{CTA} border-brand-alt bg-brand-alt text-on-brand hover:brightness-95",
                            "Read the docs"
                        }
                        if let Some(repository) = SITE_CONFIG.repository {
                            a {
                                class: "{CTA} border-line text-fg hover:border-accent",
                                href: "{repository}",
                                rel: "noreferrer",
                                target: "_blank",
                                "Source"
                            }
                        }
                    }
                }

                section { class: "mt-16 grid gap-4 [grid-template-columns:repeat(auto-fit,minmax(15rem,1fr))]",
                    Feature {
                        title: "Write markdown",
                        body: "Files in docs/ become routes, sidebar entries, and search results.",
                    }
                    Feature {
                        title: "Drop in components",
                        body: "Embedded Dioxus components keep typed props, closures, and signals.",
                    }
                    Feature {
                        title: "Ship static files",
                        body: "One build produces a site any static host will serve.",
                    }
                }
            }

            SiteFooter { config: SITE_CONFIG, pages: pages() }
        }
    }
}

/// Shared call-to-action shape; only the colours differ between the two buttons.
const CTA: &str = "inline-flex h-10 items-center rounded-lg border px-4 text-[0.925rem] \
                   font-medium no-underline";

#[component]
fn Feature(title: String, body: String) -> Element {
    rsx! {
        div { class: "rounded-lg border border-line bg-surface px-5 py-4 text-left",
            h3 { class: "m-0 mb-1.5 text-base", "{title}" }
            p { class: "m-0 text-sm text-muted", "{body}" }
        }
    }
}
