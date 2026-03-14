export type PlaybackStatus = "stopped" | "buffering" | "playing" | "paused";
export type RepeatMode = "off" | "one" | "all";

export interface TrackInfo {
  track_id: string;
  title: string;
  artist: string | null;
  album: string | null;
  duration_ms: number | null;
}

export interface PlaybackState {
  status: PlaybackStatus;
  track: TrackInfo | null;
  position_ms: number;
  duration_ms: number;
  volume: number;
  repeat_mode: RepeatMode;
  shuffle: boolean;
}

export interface QueueEntry {
  track_id: string;
  title: string;
  artist: string | null;
  album: string | null;
  duration_ms: number | null;
}

export interface QueueState {
  entries: QueueEntry[];
  current_index: number;
  repeat_mode: RepeatMode;
  shuffle: boolean;
  source_label: string;
}

export interface DspStageInfo {
  name: string;
  enabled: boolean;
  parameters: string;
}

export interface SignalPathInfo {
  source_codec: string;
  source_sample_rate: number;
  source_bit_depth: number;
  dsp_stages: DspStageInfo[];
  output_device: string;
  output_sample_rate: number;
  is_bit_perfect: boolean;
  quality_tier: string;
}

export interface ProgressEvent {
  position_ms: number;
  duration_ms: number;
}

export interface PlaybackStateEvent {
  status: PlaybackStatus;
  track: TrackInfo | null;
}

export interface QueueChangedEvent {
  entries: QueueEntry[];
  current_index: number;
}
