//! DSP controls: equalizer, crossfeed, ReplayGain, compressor.

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

const SECTION_STYLE: &str = "\
    background: #12121e; \
    border: 1px solid #1e1e2e; \
    border-radius: 8px; \
    padding: 16px 20px;\
";

const SECTION_TITLE: &str = "\
    font-size: 14px; \
    font-weight: bold; \
    color: #aaa; \
    text-transform: uppercase; \
    letter-spacing: 0.5px; \
    margin-bottom: 12px;\
";

const PLACEHOLDER: &str = "\
    color: #555; \
    font-size: 13px;\
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
