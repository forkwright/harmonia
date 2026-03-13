import { invoke } from "@tauri-apps/api/core";
import type {
  CompressorConfig,
  CrossfeedConfig,
  DspConfig,
  EqBand,
  EqPreset,
  ReplayGainConfig,
  VolumeConfig,
} from "../types/dsp";

export async function getDspConfig(): Promise<DspConfig> {
  return invoke<DspConfig>("get_dsp_config");
}

export async function setEqEnabled(enabled: boolean): Promise<void> {
  return invoke("set_eq_enabled", { enabled });
}

export async function setEqPreamp(preampDb: number): Promise<void> {
  return invoke("set_eq_preamp", { preampDb });
}

export async function setEqBand(
  index: number,
  band: EqBand,
): Promise<void> {
  return invoke("set_eq_band", { index, band });
}

export async function setEqBands(bands: EqBand[]): Promise<void> {
  return invoke("set_eq_bands", { bands });
}

export async function addEqBand(band: EqBand): Promise<void> {
  return invoke("add_eq_band", { band });
}

export async function removeEqBand(index: number): Promise<void> {
  return invoke("remove_eq_band", { index });
}

export async function loadEqPreset(name: string): Promise<void> {
  return invoke("load_eq_preset", { name });
}

export async function getEqPresets(): Promise<EqPreset[]> {
  return invoke<EqPreset[]>("get_eq_presets");
}

export async function resetEq(): Promise<void> {
  return invoke("reset_eq");
}

export async function setCrossfeed(config: CrossfeedConfig): Promise<void> {
  return invoke("set_crossfeed", { config });
}

export async function setReplaygain(config: ReplayGainConfig): Promise<void> {
  return invoke("set_replaygain", { config });
}

export async function setCompressor(config: CompressorConfig): Promise<void> {
  return invoke("set_compressor", { config });
}

export async function setVolume(config: VolumeConfig): Promise<void> {
  return invoke("set_volume", { config });
}
