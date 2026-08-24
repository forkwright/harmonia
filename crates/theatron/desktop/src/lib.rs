//! Dioxus desktop UI for the Harmonia media platform.

#![deny(missing_docs)]

pub(crate) mod app;
pub(crate) mod layout;
pub(crate) mod state;
pub(crate) mod theme;
pub(crate) mod tokens;
pub(crate) mod views;

/// Launch the desktop application.
pub fn run() {
    // WHY: reqwest builds with `rustls-no-provider` (fleet convention:
    // install the ring crypto provider once, explicitly, process-wide —
    // never let a library link one implicitly). install_default returns Err
    // if a provider is already installed (e.g. a dependency called it
    // first); that is harmless.
    // kanon:ignore RUST/no-silent-result-swallow — install_default returns Err when provider already installed by dependency; harmless
    let _ = rustls::crypto::ring::default_provider().install_default();

    dioxus::launch(app::App);
}
