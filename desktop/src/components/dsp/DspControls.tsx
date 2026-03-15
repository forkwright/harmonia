import { useState } from "react";
import type {
  CrossfeedConfig,
  CrossfeedPreset,
  ReplayGainConfig,
  ReplayGainMode,
  CompressorConfig,
  VolumeConfig,
  OutputDeviceInfo,
} from "../../types/dsp";

interface DspControlsProps {
  crossfeed: CrossfeedConfig;
  replaygain: ReplayGainConfig;
  compressor: CompressorConfig;
  volume: VolumeConfig;
  outputDevices: OutputDeviceInfo[];
  selectedOutputDevice: string | null;
  onCrossfeedPresetChange: (preset: CrossfeedPreset) => void;
  onReplaygainChange: (cfg: ReplayGainConfig) => void;
  onCompressorChange: (cfg: CompressorConfig) => void;
  onVolumeChange: (cfg: VolumeConfig) => void;
  onOutputDeviceChange: (deviceId: string | null) => void;
  onRefreshDevices: () => void;
}

const CROSSFEED_PRESETS: { label: string; value: CrossfeedPreset }[] = [
  { label: "None", value: "None" },
  { label: "Light", value: "Light" },
  { label: "Medium", value: "Medium" },
  { label: "Strong", value: "Strong" },
];

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(true);

  return (
    <div className="bg-gray-800 rounded-lg overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-gray-750"
      >
        <span className="text-sm font-medium text-white">{title}</span>
        <span className="text-gray-500 text-xs">{open ? "Hide" : "Show"}</span>
      </button>
      {open && <div className="px-4 pb-4 space-y-3">{children}</div>}
    </div>
  );
}

function ToggleSection({
  title,
  enabled,
  onToggle,
  children,
}: {
  title: string;
  enabled: boolean;
  onToggle: (v: boolean) => void;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(true);

  return (
    <div className="bg-gray-800 rounded-lg overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-gray-750"
      >
        <div className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => {
              e.stopPropagation();
              onToggle(e.target.checked);
            }}
            onClick={(e) => e.stopPropagation()}
            className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
          />
          <span
            className={`text-sm font-medium ${enabled ? "text-white" : "text-gray-500"}`}
          >
            {title}
          </span>
        </div>
        <span className="text-gray-500 text-xs">{open ? "Hide" : "Show"}</span>
      </button>
      {open && <div className="px-4 pb-4 space-y-3">{children}</div>}
    </div>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  onChange: (v: number) => void;
}) {
  return (
    <div className="space-y-1">
      <div className="flex justify-between">
        <label className="text-xs text-gray-400">{label}</label>
        <span className="text-xs font-mono text-gray-400">
          {value.toFixed(step < 1 ? 1 : 0)} {unit}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-indigo-500"
      />
    </div>
  );
}

export default function DspControls({
  crossfeed,
  replaygain,
  compressor,
  volume,
  outputDevices,
  selectedOutputDevice,
  onCrossfeedPresetChange,
  onReplaygainChange,
  onCompressorChange,
  onVolumeChange,
  onOutputDeviceChange,
  onRefreshDevices,
}: DspControlsProps) {
  return (
    <div className="space-y-3">
      {/* Output Device */}
      <Section title="Output Device">
        <div className="flex items-center gap-2">
          <select
            value={selectedOutputDevice ?? ""}
            onChange={(e) => onOutputDeviceChange(e.target.value || null)}
            className="flex-1 bg-gray-700 text-sm text-gray-200 rounded px-3 py-1.5 border border-gray-600 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 focus:outline-none"
          >
            <option value="">System Default</option>
            {outputDevices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name}
                {device.is_default ? " (default)" : ""}
              </option>
            ))}
          </select>
          <button
            onClick={onRefreshDevices}
            className="px-2 py-1.5 text-xs bg-gray-700 text-gray-400 hover:text-white rounded border border-gray-600 transition-colors"
            title="Refresh devices"
          >
            Refresh
          </button>
        </div>
      </Section>

      {/* Crossfeed */}
      <Section title="Crossfeed">
        <p className="text-xs text-gray-500">
          Blends stereo channels for natural headphone listening.
        </p>
        <div className="flex gap-2">
          {CROSSFEED_PRESETS.map(({ label, value }) => (
            <button
              key={value}
              onClick={() => onCrossfeedPresetChange(value)}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                crossfeed.preset === value
                  ? "bg-indigo-600 text-white"
                  : "bg-gray-700 text-gray-400 hover:text-white"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </Section>

      {/* ReplayGain */}
      <Section title="ReplayGain">
        <p className="text-xs text-gray-500">
          Normalizes loudness across tracks using embedded gain tags.
        </p>
        <div className="space-y-1">
          <label className="text-xs text-gray-400">Mode</label>
          <div className="flex gap-2">
            <button
              onClick={() =>
                onReplaygainChange({ ...replaygain, enabled: false })
              }
              className={`px-3 py-1 text-xs rounded transition-colors ${
                !replaygain.enabled
                  ? "bg-indigo-600 text-white"
                  : "bg-gray-700 text-gray-400 hover:text-white"
              }`}
            >
              Off
            </button>
            {(["Track", "Album"] as ReplayGainMode[]).map((mode) => (
              <button
                key={mode}
                onClick={() =>
                  onReplaygainChange({
                    ...replaygain,
                    enabled: true,
                    mode,
                  })
                }
                className={`px-3 py-1 text-xs rounded transition-colors ${
                  replaygain.enabled && replaygain.mode === mode
                    ? "bg-indigo-600 text-white"
                    : "bg-gray-700 text-gray-400 hover:text-white"
                }`}
              >
                {mode}
              </button>
            ))}
          </div>
        </div>
        {replaygain.enabled && (
          <>
            <Slider
              label="Preamp"
              value={replaygain.preamp_db}
              min={-12}
              max={12}
              step={0.5}
              unit="dB"
              onChange={(v) =>
                onReplaygainChange({ ...replaygain, preamp_db: v })
              }
            />
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={replaygain.prevent_clipping}
                onChange={(e) =>
                  onReplaygainChange({
                    ...replaygain,
                    prevent_clipping: e.target.checked,
                  })
                }
                className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
              />
              Prevent clipping
            </label>
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={replaygain.fallback_to_track}
                onChange={(e) =>
                  onReplaygainChange({
                    ...replaygain,
                    fallback_to_track: e.target.checked,
                  })
                }
                className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
              />
              Fall back to track gain when album gain unavailable
            </label>
          </>
        )}
      </Section>

      {/* Compressor */}
      <ToggleSection
        title="Compressor"
        enabled={compressor.enabled}
        onToggle={(v) => onCompressorChange({ ...compressor, enabled: v })}
      >
        <p className="text-xs text-gray-500">
          Reduces dynamic range for late-night listening.
        </p>
        <Slider
          label="Threshold"
          value={compressor.threshold_db}
          min={-30}
          max={0}
          step={0.5}
          unit="dB"
          onChange={(v) =>
            onCompressorChange({ ...compressor, threshold_db: v })
          }
        />
        <Slider
          label="Ratio"
          value={compressor.ratio}
          min={1}
          max={20}
          step={0.5}
          unit=":1"
          onChange={(v) => onCompressorChange({ ...compressor, ratio: v })}
        />
        <Slider
          label="Attack"
          value={compressor.attack_ms}
          min={0.1}
          max={100}
          step={0.1}
          unit="ms"
          onChange={(v) =>
            onCompressorChange({ ...compressor, attack_ms: v })
          }
        />
        <Slider
          label="Release"
          value={compressor.release_ms}
          min={10}
          max={1000}
          step={10}
          unit="ms"
          onChange={(v) =>
            onCompressorChange({ ...compressor, release_ms: v })
          }
        />
        <Slider
          label="Limiter Ceiling"
          value={compressor.limiter_ceiling_db}
          min={-6}
          max={0}
          step={0.1}
          unit="dB"
          onChange={(v) =>
            onCompressorChange({ ...compressor, limiter_ceiling_db: v })
          }
        />
      </ToggleSection>

      {/* Volume */}
      <Section title="Volume">
        <Slider
          label="Level"
          value={volume.level_db}
          min={-60}
          max={0}
          step={0.5}
          unit="dB"
          onChange={(v) => onVolumeChange({ ...volume, level_db: v })}
        />
        <label className="flex items-center gap-2 text-xs text-gray-400">
          <input
            type="checkbox"
            checked={volume.dither}
            onChange={(e) =>
              onVolumeChange({ ...volume, dither: e.target.checked })
            }
            className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
          />
          TPDF dither
        </label>
      </Section>
    </div>
  );
}
