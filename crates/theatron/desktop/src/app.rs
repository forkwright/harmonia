//! Root component with router and connection gating.

use dioxus::prelude::*;

use crate::layout::Layout;
use crate::state::AppState;
use crate::theme::ThemeProvider;
use crate::views::dsp::Dsp;
use crate::views::library::Library;
use crate::views::now_playing::NowPlaying;
use crate::views::settings::Settings;

#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub(crate) enum Route {
    #[layout(Layout)]
        #[route("/")]
        NowPlaying {},
        #[route("/library")]
        Library {},
        #[route("/dsp")]
        Dsp {},
        #[route("/settings")]
        Settings {},
}

/// Root component.
///
/// Provides global app state as context, then renders the router
/// wrapped in the theme provider.
#[component]
pub(crate) fn App() -> Element {
    let app_state = use_signal(AppState::default);
    use_context_provider(|| app_state);

    rsx! {
        ThemeProvider {
            Router::<Route> {}
        }
    }
}
