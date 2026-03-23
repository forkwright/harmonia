//! Now playing view: transport controls, progress bar, queue.

use dioxus::prelude::*;

const CONTAINER_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    align-items: center; \
    justify-content: center; \
    height: 100%; \
    gap: 24px;\
";

const TITLE_STYLE: &str = "\
    font-size: 24px; \
    font-weight: bold; \
    color: #ffffff;\
";

const SUBTITLE_STYLE: &str = "\
    font-size: 14px; \
    color: #666;\
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
