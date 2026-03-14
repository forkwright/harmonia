/** Podcast-specific playback controls: skip 15s back, play/pause, skip 30s forward. */

import { usePodcastPlayback } from "../hooks/usePodcastPlayback";
import { usePodcastStore } from "../store";

export default function PodcastTransport() {
  const { isPlaying, currentEpisodeId, pause, resumeEpisode, skipForward, skipBackward } =
    usePodcastPlayback();
  const positionMs = usePodcastStore((s) => s.positionMs);

  const disabled = !currentEpisodeId;

  const handlePlayPause = () => {
    if (isPlaying) {
      void pause();
    } else if (currentEpisodeId) {
      void resumeEpisode(currentEpisodeId, positionMs);
    }
  };

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={() => void skipBackward()}
        disabled={disabled}
        className="flex flex-col items-center px-2 py-1 rounded text-gray-400 hover:text-gray-100 disabled:opacity-40 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
        aria-label="Skip backward 15 seconds"
        title="← 15s"
      >
        <span className="text-lg leading-none">↩</span>
        <span className="text-[10px] font-medium">15s</span>
      </button>

      <button
        onClick={handlePlayPause}
        disabled={disabled}
        className="w-10 h-10 rounded-full bg-blue-600 hover:bg-blue-500 disabled:opacity-40 disabled:cursor-not-allowed flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-blue-400"
        aria-label={isPlaying ? "Pause" : "Play"}
      >
        <span className="text-white text-sm">{isPlaying ? "⏸" : "▶"}</span>
      </button>

      <button
        onClick={() => void skipForward()}
        disabled={disabled}
        className="flex flex-col items-center px-2 py-1 rounded text-gray-400 hover:text-gray-100 disabled:opacity-40 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
        aria-label="Skip forward 30 seconds"
        title="30s →"
      >
        <span className="text-lg leading-none">↪</span>
        <span className="text-[10px] font-medium">30s</span>
      </button>
    </div>
  );
}
