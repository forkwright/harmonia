import { useState, useEffect, useCallback, useRef } from "react";
import type {
  DspConfig,
  EqBand,
  EqPreset,
  CrossfeedConfig,
  ReplayGainConfig,
  CompressorConfig,
  VolumeConfig,
} from "../types/dsp";
import * as dspApi from "../api/dsp";

export function useDsp() {
  const [config, setConfig] = useState<DspConfig | null>(null);
  const [presets, setPresets] = useState<EqPreset[]>([]);
  const [loading, setLoading] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    Promise.all([dspApi.getDspConfig(), dspApi.getEqPresets()])
      .then(([cfg, p]) => {
        setConfig(cfg);
        setPresets(p);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const refresh = useCallback(async () => {
    const cfg = await dspApi.getDspConfig();
    setConfig(cfg);
  }, []);

  const debouncedRefresh = useCallback(() => {
    clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(refresh, 50);
  }, [refresh]);

  const setEqEnabled = useCallback(
    async (enabled: boolean) => {
      await dspApi.setEqEnabled(enabled);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const setEqPreamp = useCallback(
    async (preampDb: number) => {
      await dspApi.setEqPreamp(preampDb);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const setEqBand = useCallback(
    async (index: number, band: EqBand) => {
      await dspApi.setEqBand(index, band);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const setEqBands = useCallback(
    async (bands: EqBand[]) => {
      await dspApi.setEqBands(bands);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const addEqBand = useCallback(
    async (band: EqBand) => {
      await dspApi.addEqBand(band);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const removeEqBand = useCallback(
    async (index: number) => {
      await dspApi.removeEqBand(index);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const loadPreset = useCallback(
    async (name: string) => {
      await dspApi.loadEqPreset(name);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const resetEq = useCallback(async () => {
    await dspApi.resetEq();
    debouncedRefresh();
  }, [debouncedRefresh]);

  const setCrossfeed = useCallback(
    async (cfg: CrossfeedConfig) => {
      await dspApi.setCrossfeed(cfg);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const setReplaygain = useCallback(
    async (cfg: ReplayGainConfig) => {
      await dspApi.setReplaygain(cfg);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const setCompressor = useCallback(
    async (cfg: CompressorConfig) => {
      await dspApi.setCompressor(cfg);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const setVolume = useCallback(
    async (cfg: VolumeConfig) => {
      await dspApi.setVolume(cfg);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  return {
    config,
    presets,
    loading,
    setEqEnabled,
    setEqPreamp,
    setEqBand,
    setEqBands,
    addEqBand,
    removeEqBand,
    loadPreset,
    resetEq,
    setCrossfeed,
    setReplaygain,
    setCompressor,
    setVolume,
  };
}
