use akouo_core::output::OutputBackend;
use akouo_core::output::cpal::CpalOutputBackend;
use serde::Serialize;
use tauri::State;

use super::DspController;
use super::config::{
    CompressorConfig, CrossfeedConfig, CrossfeedPreset, DspConfig, EqBand, ReplayGainConfig,
    VolumeConfig,
};
use super::presets::{EqPreset, built_in_presets};

#[derive(Debug, Clone, Serialize)]
pub struct OutputDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

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
pub fn set_crossfeed_preset(preset: CrossfeedPreset, controller: State<'_, DspController>) {
    controller.update_config(|cfg| {
        cfg.crossfeed.preset = preset;
        cfg.crossfeed.enabled = preset.enabled();
        cfg.crossfeed.strength = preset.strength();
    });
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

#[tauri::command]
pub fn list_output_devices() -> Result<Vec<OutputDeviceInfo>, String> {
    let backend = CpalOutputBackend::new();
    let devices = backend.available_devices().map_err(|e| e.to_string())?;
    Ok(devices
        .into_iter()
        .map(|d| OutputDeviceInfo {
            id: d.id,
            name: d.name,
            is_default: d.is_default,
        })
        .collect())
}

#[tauri::command]
pub fn set_output_device(device_id: Option<String>, controller: State<'_, DspController>) {
    controller.update_config(|cfg| cfg.output_device = device_id);
}
