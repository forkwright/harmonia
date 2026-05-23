//! Now playing view: transport controls, progress bar, queue.

use dioxus::prelude::*;

const CONTAINER_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    align-items: center; \
    justify-content: center; \
    height: 100%; \
    gap: var(--space-6);\
";

const TITLE_STYLE: &str = "\
    font-size: var(--text-xl); \
    font-weight: var(--weight-bold); \
    color: var(--text-primary);\
";

const SUBTITLE_STYLE: &str = "\
    font-size: var(--text-base); \
    color: var(--text-muted);\
";

/// Now playing view stub.
#[component]
pub(crate) fn NowPlaying() -> Element {
    rsx! {
        div {
            style: "{CONTAINER_STYLE}",
            div { style: "{TITLE_STYLE}", "Now Playing" }
            div { style: "{SUBTITLE_STYLE}", "Transport controls, progress bar, and queue will appear here." }
        }
    }
}
