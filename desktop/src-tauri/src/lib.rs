mod commands;
mod config;
mod dsp;
mod playback;

use playback::PlaybackEngine;

pub fn run() {
    let engine = PlaybackEngine::new().expect("audio engine failed to initialise");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(dsp::DspController::new())
        .manage(playback::podcast::PodcastController::new())
        .manage(playback::audiobook::AudiobookController::new())
        .manage(engine)
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
            playback::podcast::podcast_play_episode,
            playback::podcast::podcast_resume_episode,
            playback::podcast::podcast_set_speed,
            playback::podcast::podcast_get_speed,
            playback::podcast::podcast_skip_forward,
            playback::podcast::podcast_skip_backward,
            playback::podcast::podcast_set_trim_silence,
            playback::podcast::podcast_get_playback_snapshot,
            playback::audiobook::audiobook_play,
            playback::audiobook::audiobook_play_from_chapter,
            playback::audiobook::audiobook_resume,
            playback::audiobook::audiobook_pause,
            playback::audiobook::audiobook_stop,
            playback::audiobook::audiobook_next_chapter,
            playback::audiobook::audiobook_prev_chapter,
            playback::audiobook::audiobook_go_to_chapter,
            playback::audiobook::audiobook_skip_forward,
            playback::audiobook::audiobook_skip_backward,
            playback::audiobook::audiobook_set_speed,
            playback::audiobook::audiobook_get_speed,
            playback::audiobook::sleep_timer_set,
            playback::audiobook::sleep_timer_set_end_of_chapter,
            playback::audiobook::sleep_timer_cancel,
            playback::audiobook::sleep_timer_extend,
            playback::audiobook::sleep_timer_get,
            playback::audiobook::audiobook_get_position,
            playback::audiobook::audiobook_update_offset,
            playback::commands::play_track,
            playback::commands::pause,
            playback::commands::resume,
            playback::commands::stop,
            playback::commands::seek,
            playback::commands::next_track,
            playback::commands::previous_track,
            playback::commands::playback_set_volume,
            playback::commands::playback_get_volume,
            playback::commands::queue_add,
            playback::commands::queue_remove,
            playback::commands::queue_clear,
            playback::commands::queue_move,
            playback::commands::queue_get,
            playback::commands::set_repeat_mode,
            playback::commands::set_shuffle,
            playback::commands::get_playback_state,
            playback::commands::get_signal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
