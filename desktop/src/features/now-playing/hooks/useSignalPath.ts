import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { SignalPathInfo } from "../../../types/playback";

const INITIAL_PATH: SignalPathInfo = {
  source_codec: "",
  source_sample_rate: 44100,
  source_bit_depth: 16,
  dsp_stages: [],
  output_device: "",
  output_sample_rate: 44100,
  is_bit_perfect: false,
  quality_tier: "Stopped",
};

const POLL_INTERVAL_MS = 2000;

export function useSignalPath(): SignalPathInfo {
  const [info, setInfo] = useState<SignalPathInfo>(INITIAL_PATH);

  useEffect(() => {
    let cancelled = false;

    async function poll() {
      try {
        const result = await invoke<SignalPathInfo>("get_signal_path");
        if (!cancelled) setInfo(result);
      } catch {
        // Not playing — keep previous state.
      }
    }

    poll();
    const id = window.setInterval(poll, POLL_INTERVAL_MS);

    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  return info;
}
