/** Speed selector with presets and fine-grained control for podcast playback. */

import { usePodcastPlayback } from "../hooks/usePodcastPlayback";

const PRESETS = [1.0, 1.2, 1.5, 2.0];

export default function PodcastSpeedControl() {
  const { speed, changeSpeed } = usePodcastPlayback();

  return (
    <div className="flex items-center gap-1">
      {PRESETS.map((preset) => (
        <button
          key={preset}
          onClick={() => void changeSpeed(preset)}
          className={`px-2 py-1 rounded text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            Math.abs(speed - preset) < 0.01
              ? "bg-blue-600 text-white"
              : "text-gray-400 hover:text-gray-100 hover:bg-gray-700"
          }`}
        >
          {preset}×
        </button>
      ))}
    </div>
  );
}
