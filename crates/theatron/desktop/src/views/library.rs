//! Library browser: albums, tracks, audiobooks, podcasts.

use dioxus::prelude::*;

const CONTAINER_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    gap: 24px;\
";

const TITLE_STYLE: &str = "\
    font-size: 24px; \
    font-weight: bold; \
    color: #ffffff;\
";

const GRID_PLACEHOLDER: &str = "\
    display: grid; \
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); \
    gap: 16px;\
";

const CARD_STYLE: &str = "\
    background: #12121e; \
    border: 1px solid #1e1e2e; \
    border-radius: 8px; \
    padding: 16px; \
    text-align: center; \
    color: #888;\
";

/// Library browser view stub.
#[component]
pub(crate) fn Library() -> Element {
    rsx! {
        div {
            style: "{CONTAINER_STYLE}",
            div { style: "{TITLE_STYLE}", "Library" }
            div {
                style: "{GRID_PLACEHOLDER}",
                for section in ["Albums", "Tracks", "Audiobooks", "Podcasts"] {
                    div {
                        style: "{CARD_STYLE}",
                        "{section}"
                    }
                }
            }
        }
    }
}
