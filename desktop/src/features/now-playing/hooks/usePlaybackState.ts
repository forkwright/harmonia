import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type {
  PlaybackState,
  PlaybackStateEvent,
  ProgressEvent,
} from "../../../types/playback";

const INITIAL_STATE: PlaybackState = {
  status: "stopped",
  track: null,
  position_ms: 0,
  duration_ms: 0,
  volume: 1.0,
  repeat_mode: "off",
  shuffle: false,
};

export function usePlaybackState(): PlaybackState {
  const [state, setState] = useState<PlaybackState>(INITIAL_STATE);

  useEffect(() => {
    // Poll for initial state.
    invoke<PlaybackState>("get_playback_state")
      .then((s) => setState(s))
      .catch(() => {});

    // Listen for progress events (250ms interval during playback).
    const unlistenProgress = listen<ProgressEvent>(
      "playback-progress",
      (event) => {
        setState((prev) => ({
          ...prev,
          position_ms: event.payload.position_ms,
          duration_ms: event.payload.duration_ms,
        }));
      }
    );

    // Listen for state-change events (status, track metadata changes).
    const unlistenState = listen<PlaybackStateEvent>(
      "playback-state-changed",
      (event) => {
        setState((prev) => ({
          ...prev,
          status: event.payload.status,
          track: event.payload.track,
        }));
      }
    );

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenState.then((fn) => fn());
    };
  }, []);

  return state;
}
