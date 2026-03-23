//! Layout shell with sidebar navigation and content area.

use dioxus::prelude::*;

use crate::app::Route;

const SHELL_STYLE: &str = "\
    display: flex; \
    height: 100vh; \
    font-family: system-ui, -apple-system, sans-serif; \
    background: #0a0a12; \
    color: #e0e0e0;\
";

const SIDEBAR_STYLE: &str = "\
    width: 220px; \
    background: #12121e; \
    border-right: 1px solid #1e1e2e; \
    padding: 16px; \
    display: flex; \
    flex-direction: column; \
    gap: 4px; \
    flex-shrink: 0;\
";

const CONTENT_STYLE: &str = "\
    flex: 1; \
    padding: 24px; \
    overflow-y: auto; \
    background: #0a0a12;\
";

const BRAND_STYLE: &str = "\
    font-size: 18px; \
    font-weight: bold; \
    padding: 8px 12px; \
    margin-bottom: 16px; \
    color: #ffffff; \
    letter-spacing: 0.5px;\
";

const NAV_LINK_STYLE: &str = "\
    display: flex; \
    align-items: center; \
    gap: 10px; \
    padding: 10px 12px; \
    border-radius: 6px; \
    color: #a0a0b0; \
    text-decoration: none; \
    font-size: 14px; \
    transition: background 0.15s, color 0.15s;\
";

const NAV_SECTION_STYLE: &str = "\
    font-size: 11px; \
    font-weight: 600; \
    color: #555; \
    text-transform: uppercase; \
    letter-spacing: 1px; \
    padding: 16px 12px 6px 12px;\
";

/// Layout shell rendered around all routes.
#[component]
pub(crate) fn Layout() -> Element {
    rsx! {
        div {
            style: "{SHELL_STYLE}",
            nav {
                style: "{SIDEBAR_STYLE}",
                div { style: "{BRAND_STYLE}", "Harmonia" }

                div { style: "{NAV_SECTION_STYLE}", "Playback" }
                NavItem { to: Route::NowPlaying {}, icon: "\u{25B6}", label: "Now Playing" }

                div { style: "{NAV_SECTION_STYLE}", "Browse" }
                NavItem { to: Route::Library {}, icon: "\u{266B}", label: "Library" }

                div { style: "{NAV_SECTION_STYLE}", "Audio" }
                NavItem { to: Route::Dsp {}, icon: "\u{2261}", label: "DSP" }

                div { style: "{NAV_SECTION_STYLE}", "System" }
                NavItem { to: Route::Settings {}, icon: "\u{2699}", label: "Settings" }
            }
            main {
                style: "{CONTENT_STYLE}",
                Outlet::<Route> {}
            }
        }
    }
}

#[component]
fn NavItem(to: Route, icon: &'static str, label: &'static str) -> Element {
    rsx! {
        Link {
            to,
            style: "{NAV_LINK_STYLE}",
            span { "{icon}" }
            span { "{label}" }
        }
    }
}
