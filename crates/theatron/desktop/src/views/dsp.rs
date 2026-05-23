//! DSP controls: equalizer, crossfeed, ReplayGain, compressor.

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

const SECTION_STYLE: &str = "\
    background: var(--bg-surface); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-lg); \
    box-shadow: var(--shadow-card); \
    padding: var(--space-4) var(--space-5);\
";

const SECTION_TITLE: &str = "\
    font-size: var(--text-base); \
    font-weight: var(--weight-bold); \
    color: var(--text-secondary); \
    text-transform: uppercase; \
    letter-spacing: var(--tracking-wide); \
    margin-bottom: var(--space-3);\
";

const PLACEHOLDER: &str = "\
    color: var(--text-muted); \
    font-size: var(--text-sm);\
";

/// DSP controls view stub.
#[component]
pub(crate) fn Dsp() -> Element {
    rsx! {
        div {
            style: "{CONTAINER_STYLE}",
            div { style: "{TITLE_STYLE}", "DSP Controls" }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "Equalizer" }
                div { style: "{PLACEHOLDER}", "EQ curve visualization and band controls will appear here." }
            }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "Crossfeed" }
                div { style: "{PLACEHOLDER}", "Crossfeed controls will appear here." }
            }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "ReplayGain" }
                div { style: "{PLACEHOLDER}", "ReplayGain mode selector will appear here." }
            }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "Signal Path" }
                div { style: "{PLACEHOLDER}", "Full signal path visualization will appear here." }
            }
        }
    }
}
