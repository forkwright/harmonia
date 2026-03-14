use tauri::State;

use super::config::{
    CompressorConfig, CrossfeedConfig, DspConfig, EqBand, ReplayGainConfig, VolumeConfig,
};
use super::presets::{built_in_presets, EqPreset};
use super::DspController;

#[tauri::command]
pub fn get_dsp_config(controller: State<'_, DspController>) -> DspConfig {
    controller.get_config()
}

#[tauri::command]
pub fn set_eq_enabled(enabled: bool, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.eq.enabled = enabled);
}

#[tauri::command]
pub fn set_eq_preamp(preamp_db: f64, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.eq.preamp_db = preamp_db);
}

#[tauri::command]
pub fn set_eq_band(
    index: usize,
    band: EqBand,
    controller: State<'_, DspController>,
) -> Result<(), String> {
    controller.update_config(|cfg| {
        if index < cfg.eq.bands.len() {
            cfg.eq.bands[index] = band;
        }
    });
    Ok(())
}

#[tauri::command]
pub fn set_eq_bands(bands: Vec<EqBand>, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.eq.bands = bands);
}

#[tauri::command]
pub fn add_eq_band(band: EqBand, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.eq.bands.push(band));
}

#[tauri::command]
pub fn remove_eq_band(index: usize, controller: State<'_, DspController>) -> Result<(), String> {
    controller.update_config(|cfg| {
        if index < cfg.eq.bands.len() {
            cfg.eq.bands.remove(index);
        }
    });
    Ok(())
}

#[tauri::command]
pub fn load_eq_preset(name: String, controller: State<'_, DspController>) -> Result<(), String> {
    let presets = built_in_presets();
    let preset = presets
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("preset not found: {name}"))?;
    controller.update_config(|cfg| {
        cfg.eq.bands = preset.bands;
        cfg.eq.preamp_db = preset.preamp_db;
    });
    Ok(())
}

#[tauri::command]
pub fn get_eq_presets() -> Vec<EqPreset> {
    built_in_presets()
}

#[tauri::command]
pub fn reset_eq(controller: State<'_, DspController>) {
    controller.update_config(|cfg| {
        cfg.eq = super::config::EqConfig::iso_10_band();
    });
}

#[tauri::command]
pub fn set_crossfeed(config: CrossfeedConfig, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.crossfeed = config);
}

#[tauri::command]
pub fn set_replaygain(config: ReplayGainConfig, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.replaygain = config);
}

#[tauri::command]
pub fn set_compressor(config: CompressorConfig, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.compressor = config);
}

#[tauri::command]
pub fn set_volume(config: VolumeConfig, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.volume = config);
}
