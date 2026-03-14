import { useState } from "react";
import { useNowPlayingStore } from "../store";
import { usePlayback } from "../hooks/usePlayback";

export default function VolumeControl() {
  const volume = useNowPlayingStore((s) => s.volume);
  const setVolume = useNowPlayingStore((s) => s.setVolume);
  const { setVolume: setEngineVolume } = usePlayback();
  const [muted, setMuted] = useState(false);
  const [preMuteVolume, setPreMuteVolume] = useState(1.0);

  async function handleVolumeChange(e: React.ChangeEvent<HTMLInputElement>) {
    const level = Number(e.target.value);
    setVolume(level);
    if (muted && level > 0) setMuted(false);
    await setEngineVolume(level);
  }

  async function handleMuteToggle() {
    if (muted) {
      setMuted(false);
      setVolume(preMuteVolume);
      await setEngineVolume(preMuteVolume);
    } else {
      setPreMuteVolume(volume);
      setMuted(true);
      await setEngineVolume(0);
    }
  }

  const displayVolume = muted ? 0 : volume;

  return (
    <div className="flex items-center gap-1.5">
      <button
        onClick={handleMuteToggle}
        className="text-gray-400 hover:text-white transition-colors text-sm"
        aria-label={muted ? "Unmute" : "Mute"}
        title={muted ? "Unmute" : "Mute"}
      >
        {displayVolume === 0 ? "🔇" : displayVolume < 0.5 ? "🔉" : "🔊"}
      </button>
      <input
        type="range"
        min={0}
        max={1}
        step={0.01}
        value={displayVolume}
        onChange={handleVolumeChange}
        className="w-20 accent-white"
        aria-label="Volume"
      />
    </div>
  );
}
