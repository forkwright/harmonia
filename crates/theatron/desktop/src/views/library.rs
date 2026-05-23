//! Library browser: albums, tracks, audiobooks, podcasts.

use dioxus::prelude::*;

const CONTAINER_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    gap: var(--space-6);\
";

const TITLE_STYLE: &str = "\
    font-size: var(--text-xl); \
    font-weight: var(--weight-bold); \
    color: var(--text-primary);\
";

const GRID_PLACEHOLDER: &str = "\
    display: grid; \
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); \
    gap: var(--space-4);\
";

const CARD_STYLE: &str = "\
    background: var(--bg-surface); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-lg); \
    box-shadow: var(--shadow-card); \
    padding: var(--space-4); \
    text-align: center; \
    color: var(--text-muted);\
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
