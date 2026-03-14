/** Now-playing bar variant rendered when a podcast episode is active. */

import { usePodcastStore } from "../store";
import PodcastTransport from "./PodcastTransport";
import PodcastSpeedControl from "./PodcastSpeedControl";

interface Props {
  episodeTitle: string;
  podcastTitle: string;
}

export default function PodcastNowPlaying({ episodeTitle, podcastTitle }: Props) {
  const { positionMs, speed } = usePodcastStore();

  function formatPosition(ms: number): string {
    const s = Math.floor(ms / 1000);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
    return `${m}:${String(sec).padStart(2, "0")}`;
  }

  return (
    <div className="flex items-center gap-4 w-full px-2">
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-gray-100 truncate">{episodeTitle}</p>
        <p className="text-xs text-gray-400 truncate">{podcastTitle}</p>
      </div>

      <PodcastTransport />

      <div className="flex items-center gap-3 flex-shrink-0">
        <span className="text-xs text-gray-400 font-mono">{formatPosition(positionMs)}</span>
        <span className="text-xs font-semibold text-blue-400">{speed.toFixed(1)}×</span>
        <PodcastSpeedControl />
      </div>
    </div>
  );
}
