export type FilterType =
  | "Peaking"
  | "LowShelf"
  | "HighShelf"
  | "LowPass"
  | "HighPass"
  | "Notch"
  | "AllPass";

export interface EqBand {
  frequency: number;
  gain_db: number;
  q: number;
  filter_type: FilterType;
  enabled: boolean;
}

export interface EqConfig {
  enabled: boolean;
  preamp_db: number;
  bands: EqBand[];
}

export interface CrossfeedConfig {
  enabled: boolean;
  strength: number;
}

export type ReplayGainMode = "Track" | "Album";

export interface ReplayGainConfig {
  enabled: boolean;
  mode: ReplayGainMode;
  preamp_db: number;
  fallback_to_track: boolean;
  prevent_clipping: boolean;
}

export interface CompressorConfig {
  enabled: boolean;
  threshold_db: number;
  ratio: number;
  attack_ms: number;
  release_ms: number;
  limiter_ceiling_db: number;
}

export interface VolumeConfig {
  level_db: number;
  dither: boolean;
}

export interface DspConfig {
  eq: EqConfig;
  crossfeed: CrossfeedConfig;
  replaygain: ReplayGainConfig;
  compressor: CompressorConfig;
  volume: VolumeConfig;
}

export interface EqPreset {
  name: string;
  bands: EqBand[];
  preamp_db: number;
}
