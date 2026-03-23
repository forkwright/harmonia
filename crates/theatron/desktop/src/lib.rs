//! Dioxus desktop UI for the Harmonia media platform.

pub(crate) mod app;
pub(crate) mod layout;
pub(crate) mod state;
pub(crate) mod theme;
pub(crate) mod views;

/// Launch the desktop application.
pub fn run() {
    dioxus::launch(app::App);
}
