//! Settings panel: connection, appearance, audio, about.

use dioxus::prelude::*;

use crate::state::AppState;
use crate::theme::ThemeMode;

const CONTAINER_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    gap: var(--space-6); \
    max-width: 600px;\
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

const ROW_STYLE: &str = "\
    display: flex; \
    justify-content: space-between; \
    align-items: center; \
    padding: var(--space-2) 0; \
    border-bottom: 1px solid var(--border-separator);\
";

const LABEL_STYLE: &str = "\
    color: var(--text-muted); \
    font-size: var(--text-sm);\
";

const VALUE_STYLE: &str = "\
    color: var(--text-primary); \
    font-size: var(--text-sm);\
";

const TOGGLE_BTN: &str = "\
    background: var(--bg-surface-bright); \
    color: var(--text-primary); \
    border: 1px solid var(--input-border); \
    border-radius: var(--radius-lg); \
    padding: var(--space-2) var(--space-4); \
    font-size: var(--text-sm); \
    cursor: pointer;\
";

const ABOUT_STYLE: &str = "\
    color: var(--text-muted); \
    font-size: var(--text-xs); \
    line-height: var(--leading-relaxed);\
";

/// Settings view.
#[component]
pub(crate) fn Settings() -> Element {
    let app_state: Signal<AppState> = use_context();
    let mut theme_mode: Signal<ThemeMode> = use_context();

    let server_url = app_state.read().server_url.clone();
    let has_token = app_state.read().auth_token.is_some();
    let current_theme = *theme_mode.read();

    rsx! {
        div {
            style: "{CONTAINER_STYLE}",
            h2 { style: "font-size: var(--text-lg); margin: 0; color: var(--text-primary);", "Settings" }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "Connection" }
                div {
                    style: "{ROW_STYLE}",
                    span { style: "{LABEL_STYLE}", "Server URL" }
                    span { style: "{VALUE_STYLE}", "{server_url}" }
                }
                div {
                    style: "{ROW_STYLE} border-bottom: none;",
                    span { style: "{LABEL_STYLE}", "Auth token" }
                    span { style: "{VALUE_STYLE}",
                        if has_token { "configured" } else { "none" }
                    }
                }
            }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "Appearance" }
                div {
                    style: "{ROW_STYLE} border-bottom: none;",
                    span { style: "{LABEL_STYLE}", "Theme" }
                    button {
                        style: "{TOGGLE_BTN}",
                        onclick: move |_| {
                            theme_mode.set(current_theme.next());
                        },
                        "{current_theme.icon()} {current_theme.label()}"
                    }
                }
            }

            div {
                style: "{SECTION_STYLE}",
                div { style: "{SECTION_TITLE}", "About" }
                div {
                    style: "{ABOUT_STYLE}",
                    p { "Harmonia Desktop" }
                    p { "Self-hosted media platform" }
                    p { "Built with Dioxus + skene" }
                }
            }
        }
    }
}
