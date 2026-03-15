import { useState, useEffect, useCallback, useRef } from "react";
import type {
  DspConfig,
  EqBand,
  EqPreset,
  CrossfeedPreset,
  ReplayGainConfig,
  CompressorConfig,
  VolumeConfig,
  OutputDeviceInfo,
} from "../types/dsp";
import * as dspApi from "../api/dsp";

export function useDsp() {
  const [config, setConfig] = useState<DspConfig | null>(null);
  const [presets, setPresets] = useState<EqPreset[]>([]);
  const [outputDevices, setOutputDevices] = useState<OutputDeviceInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );

  useEffect(() => {
    Promise.all([
      dspApi.getDspConfig(),
      dspApi.getEqPresets(),
      dspApi.listOutputDevices(),
    ])
      .then(([cfg, p, devices]) => {
        setConfig(cfg);
        setPresets(p);
        setOutputDevices(devices);
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

  const setCrossfeedPreset = useCallback(
    async (preset: CrossfeedPreset) => {
      await dspApi.setCrossfeedPreset(preset);
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

  const setOutputDevice = useCallback(
    async (deviceId: string | null) => {
      await dspApi.setOutputDevice(deviceId);
      debouncedRefresh();
    },
    [debouncedRefresh],
  );

  const refreshOutputDevices = useCallback(async () => {
    const devices = await dspApi.listOutputDevices();
    setOutputDevices(devices);
  }, []);

  return {
    config,
    presets,
    outputDevices,
    loading,
    setEqEnabled,
    setEqPreamp,
    setEqBand,
    setEqBands,
    addEqBand,
    removeEqBand,
    loadPreset,
    resetEq,
    setCrossfeedPreset,
    setReplaygain,
    setCompressor,
    setVolume,
    setOutputDevice,
    refreshOutputDevices,
  };
}
