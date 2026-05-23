//! Layout shell with sidebar navigation and content area.

use dioxus::prelude::*;

use crate::app::Route;

const SHELL_STYLE: &str = "\
    display: flex; \
    height: 100vh; \
    font-family: var(--font-sans); \
    background: var(--bg); \
    color: var(--text-primary);\
";

const SIDEBAR_STYLE: &str = "\
    width: 220px; \
    background: var(--bg-surface); \
    border-right: 1px solid var(--border); \
    padding: var(--space-4); \
    display: flex; \
    flex-direction: column; \
    gap: var(--space-1); \
    flex-shrink: 0;\
";

const CONTENT_STYLE: &str = "\
    flex: 1; \
    padding: var(--space-6); \
    overflow-y: auto; \
    background: var(--bg);\
";

const BRAND_STYLE: &str = "\
    font-size: var(--text-lg); \
    font-weight: var(--weight-bold); \
    padding: var(--space-2) var(--space-3); \
    margin-bottom: var(--space-4); \
    color: var(--text-primary); \
    letter-spacing: var(--tracking-wide);\
";

const NAV_LINK_STYLE: &str = "\
    display: flex; \
    align-items: center; \
    gap: var(--space-3); \
    padding: var(--space-2) var(--space-3); \
    border-radius: var(--radius-lg); \
    color: var(--text-secondary); \
    text-decoration: none; \
    font-size: var(--text-base); \
    transition: background var(--duration-fast) var(--ease-in-out), color var(--duration-fast) var(--ease-in-out);\
";

const NAV_SECTION_STYLE: &str = "\
    font-size: var(--text-xs); \
    font-weight: var(--weight-semibold); \
    color: var(--text-muted); \
    text-transform: uppercase; \
    letter-spacing: var(--tracking-wide); \
    padding: var(--space-4) var(--space-3) var(--space-2) var(--space-3);\
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
