import type { PlaybackStatus, RepeatMode } from "../../../types/playback";
import { usePlayback } from "../hooks/usePlayback";

interface Props {
  status: PlaybackStatus;
  shuffle: boolean;
  repeatMode: RepeatMode;
}

const REPEAT_CYCLE: RepeatMode[] = ["off", "all", "one"];

export default function TransportControls({ status, shuffle, repeatMode }: Props) {
  const { pause, resume, stop: stopPlayback, nextTrack, previousTrack, setShuffle, setRepeatMode } =
    usePlayback();

  async function handlePlayPause() {
    if (status === "playing") {
      await pause();
    } else if (status === "paused") {
      await resume();
    }
  }

  async function handleRepeat() {
    const current = REPEAT_CYCLE.indexOf(repeatMode);
    const next = REPEAT_CYCLE[(current + 1) % REPEAT_CYCLE.length];
    await setRepeatMode(next);
  }

  const isPlaying = status === "playing";

  return (
    <div className="flex items-center gap-3">
      <button
        onClick={() => setShuffle(!shuffle)}
        className={`p-1 rounded transition-colors ${
          shuffle ? "text-blue-400 hover:text-blue-300" : "text-gray-500 hover:text-gray-300"
        }`}
        title="Shuffle"
        aria-label="Toggle shuffle"
      >
        ⇄
      </button>

      <button
        onClick={() => previousTrack()}
        className="p-1 text-gray-300 hover:text-white transition-colors"
        title="Previous"
        aria-label="Previous track"
      >
        ⏮
      </button>

      <button
        onClick={handlePlayPause}
        disabled={status === "stopped" || status === "buffering"}
        className="w-8 h-8 flex items-center justify-center bg-white text-gray-900 rounded-full hover:bg-gray-200 transition-colors disabled:opacity-40"
        title={isPlaying ? "Pause" : "Play"}
        aria-label={isPlaying ? "Pause" : "Play"}
      >
        {isPlaying ? "⏸" : "▶"}
      </button>

      <button
        onClick={() => nextTrack()}
        className="p-1 text-gray-300 hover:text-white transition-colors"
        title="Next"
        aria-label="Next track"
      >
        ⏭
      </button>

      <button
        onClick={handleRepeat}
        className={`p-1 rounded transition-colors text-sm ${
          repeatMode !== "off"
            ? "text-blue-400 hover:text-blue-300"
            : "text-gray-500 hover:text-gray-300"
        }`}
        title={`Repeat: ${repeatMode}`}
        aria-label="Cycle repeat mode"
      >
        {repeatMode === "one" ? "🔂" : "🔁"}
      </button>
    </div>
  );
}
