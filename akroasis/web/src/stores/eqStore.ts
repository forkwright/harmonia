// EQ state — 10-band parametric equalizer presets and band gains
import { create } from 'zustand'

const BAND_COUNT = 10;
const GAIN_MIN = -12;
const GAIN_MAX = 12;

function flat(): number[] {
  return new Array(BAND_COUNT).fill(0) as number[];
}

const BUILT_IN_PRESETS: Record<string, number[]> = {
  Flat:       [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  Rock:       [4, 3, 2, 0, -1, -1, 0, 2, 3, 4],
  Jazz:       [3, 2, 1, 2, -1, -1, 0, 1, 2, 3],
  Classical:  [4, 3, 2, 1, 0, 0, -1, -1, -2, -3],
  Pop:        [-1, 2, 4, 3, 1, 0, 0, 1, 2, 2],
  'Bass Boost': [6, 5, 4, 2, 1, 0, 0, 0, 0, 0],
};

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function clampGain(dB: number): number {
  return Math.max(GAIN_MIN, Math.min(GAIN_MAX, dB));
}

interface EqState {
  enabled: boolean;
  bands: number[];
  activePreset: string | null;
  customPresets: Record<string, number[]>;

  setBand: (index: number, dB: number) => void;
  setPreset: (name: string) => void;
  saveCustomPreset: (name: string) => void;
  deleteCustomPreset: (name: string) => void;
  setEnabled: (enabled: boolean) => void;
  reset: () => void;
}

export const useEqStore = create<EqState>((set, get) => ({
  enabled: loadJson<boolean>('akroasis_eq_enabled', true),
  bands: loadJson<number[]>('akroasis_eq_bands', flat()),
  activePreset: null,
  customPresets: loadJson<Record<string, number[]>>('akroasis_eq_presets', {}),

  setBand: (index, dB) => {
    if (index < 0 || index >= BAND_COUNT) return;
    const bands = [...get().bands];
    bands[index] = clampGain(dB);
    localStorage.setItem('akroasis_eq_bands', JSON.stringify(bands));
    set({ bands, activePreset: null });
  },

  setPreset: (name) => {
    const { customPresets } = get();
    const gains = BUILT_IN_PRESETS[name] ?? customPresets[name];
    if (!gains) return;
    const bands = gains.map(clampGain);
    localStorage.setItem('akroasis_eq_bands', JSON.stringify(bands));
    set({ bands, activePreset: name });
  },

  saveCustomPreset: (name) => {
    const { bands, customPresets } = get();
    const updated = { ...customPresets, [name]: [...bands] };
    localStorage.setItem('akroasis_eq_presets', JSON.stringify(updated));
    set({ customPresets: updated, activePreset: name });
  },

  deleteCustomPreset: (name) => {
    const { customPresets, activePreset } = get();
    const updated = { ...customPresets };
    delete updated[name];
    localStorage.setItem('akroasis_eq_presets', JSON.stringify(updated));
    set({
      customPresets: updated,
      activePreset: activePreset === name ? null : activePreset,
    });
  },

  setEnabled: (enabled) => {
    localStorage.setItem('akroasis_eq_enabled', JSON.stringify(enabled));
    set({ enabled });
  },

  reset: () => {
    const bands = flat();
    localStorage.setItem('akroasis_eq_bands', JSON.stringify(bands));
    set({ bands, activePreset: 'Flat' });
  },
}));

export { BUILT_IN_PRESETS };
