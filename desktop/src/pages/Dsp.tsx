import { useState, useCallback } from "react";
import { useDsp } from "../hooks/useDsp";
import type { EqBand } from "../types/dsp";
import EqCurve from "../components/dsp/EqCurve";
import EqBandControls from "../components/dsp/EqBandControls";
import EqToolbar from "../components/dsp/EqToolbar";
import DspControls from "../components/dsp/DspControls";

export default function Dsp() {
  const {
    config,
    presets,
    outputDevices,
    loading,
    setEqEnabled,
    setEqPreamp,
    setEqBand,
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
  } = useDsp();

  const [selectedBand, setSelectedBand] = useState<number | null>(null);

  const handleBandChange = useCallback(
    (index: number, updates: Partial<EqBand>) => {
      if (!config) return;
      const band = { ...config.eq.bands[index], ...updates };
      setEqBand(index, band);
    },
    [config, setEqBand],
  );

  const handleAddBand = useCallback(() => {
    addEqBand({
      frequency: 1000,
      gain_db: 0,
      q: 1.414,
      filter_type: "Peaking",
      enabled: true,
    });
  }, [addEqBand]);

  const handleRemoveBand = useCallback(
    (index: number) => {
      if (selectedBand === index) setSelectedBand(null);
      else if (selectedBand !== null && selectedBand > index) {
        setSelectedBand(selectedBand - 1);
      }
      removeEqBand(index);
    },
    [removeEqBand, selectedBand],
  );

  if (loading || !config) {
    return (
      <div className="p-8">
        <p className="text-gray-400">Loading DSP configuration...</p>
      </div>
    );
  }

  return (
    <div className="p-6 pb-24 space-y-6 max-w-5xl">
      <h1 className="text-2xl font-bold">DSP</h1>

      {/* EQ Section */}
      <section className="space-y-4">
        <EqToolbar
          enabled={config.eq.enabled}
          preampDb={config.eq.preamp_db}
          presets={presets}
          onToggle={setEqEnabled}
          onPreampChange={setEqPreamp}
          onPresetLoad={loadPreset}
          onAddBand={handleAddBand}
          onReset={resetEq}
        />

        <EqCurve
          bands={config.eq.bands}
          preampDb={config.eq.preamp_db}
          selectedIndex={selectedBand}
          onBandChange={handleBandChange}
          onBandSelect={setSelectedBand}
        />

        {/* Band controls */}
        {selectedBand !== null && selectedBand < config.eq.bands.length && (
          <EqBandControls
            band={config.eq.bands[selectedBand]}
            index={selectedBand}
            onChange={handleBandChange}
            onRemove={handleRemoveBand}
          />
        )}

        {/* Band selector pills */}
        <div className="flex gap-1.5 flex-wrap">
          {config.eq.bands.map((band, i) => (
            <button
              key={i}
              onClick={() =>
                setSelectedBand(selectedBand === i ? null : i)
              }
              className={`px-2.5 py-1 text-xs rounded-full transition-colors ${
                selectedBand === i
                  ? "bg-indigo-600 text-white"
                  : band.enabled
                    ? "bg-gray-700 text-gray-300 hover:bg-gray-600"
                    : "bg-gray-800 text-gray-500 hover:bg-gray-700"
              }`}
            >
              {band.frequency >= 1000
                ? `${(band.frequency / 1000).toFixed(band.frequency % 1000 === 0 ? 0 : 1)}k`
                : band.frequency}
            </button>
          ))}
        </div>
      </section>

      {/* DSP Controls */}
      <section className="space-y-3">
        <h2 className="text-lg font-semibold text-gray-300">Controls</h2>
        <DspControls
          crossfeed={config.crossfeed}
          replaygain={config.replaygain}
          compressor={config.compressor}
          volume={config.volume}
          outputDevices={outputDevices}
          selectedOutputDevice={config.output_device}
          onCrossfeedPresetChange={setCrossfeedPreset}
          onReplaygainChange={setReplaygain}
          onCompressorChange={setCompressor}
          onVolumeChange={setVolume}
          onOutputDeviceChange={setOutputDevice}
          onRefreshDevices={refreshOutputDevices}
        />
      </section>
    </div>
  );
}
