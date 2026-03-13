mod commands;
mod config;
mod dsp;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(dsp::DspController::new())
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::get_server_url,
            commands::set_server_url,
            dsp::commands::get_dsp_config,
            dsp::commands::set_eq_enabled,
            dsp::commands::set_eq_preamp,
            dsp::commands::set_eq_band,
            dsp::commands::set_eq_bands,
            dsp::commands::add_eq_band,
            dsp::commands::remove_eq_band,
            dsp::commands::load_eq_preset,
            dsp::commands::get_eq_presets,
            dsp::commands::reset_eq,
            dsp::commands::set_crossfeed,
            dsp::commands::set_replaygain,
            dsp::commands::set_compressor,
            dsp::commands::set_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
