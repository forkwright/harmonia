/** Single episode row: title, date, duration, played/downloaded status. */

import type { Episode } from "../../../types/media";
import DownloadProgress from "./DownloadProgress";

interface Props {
  episode: Episode;
  onPlay: (id: string) => void;
}

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

export default function EpisodeRow({ episode, onPlay }: Props) {
  const isCompleted = episode.progress?.completed ?? false;
  const isInProgress = !isCompleted && (episode.progress?.positionMs ?? 0) > 0;

  return (
    <div
      className={`flex items-start gap-3 px-4 py-3 border-b border-gray-800 hover:bg-gray-800/50 transition-colors ${
        isCompleted ? "opacity-60" : ""
      }`}
    >
      <button
        onClick={() => onPlay(episode.id)}
        className="flex-shrink-0 w-8 h-8 mt-0.5 rounded-full bg-blue-600 hover:bg-blue-500 flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-blue-400"
        aria-label={`Play ${episode.title}`}
      >
        <span className="text-white text-xs ml-0.5">▶</span>
      </button>

      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-gray-100 truncate">{episode.title}</p>
        <div className="flex items-center gap-2 mt-0.5 text-xs text-gray-400">
          <span>{formatDate(episode.publishedAt)}</span>
          <span>·</span>
          <span>{formatDuration(episode.durationMs)}</span>
          {isInProgress && (
            <>
              <span>·</span>
              <span className="text-blue-400">
                {Math.round(episode.progress!.percentComplete)}%
              </span>
            </>
          )}
          {isCompleted && (
            <>
              <span>·</span>
              <span className="text-green-500">Played</span>
            </>
          )}
        </div>
        {isInProgress && episode.progress && (
          <div className="mt-1.5 h-0.5 bg-gray-700 rounded">
            <div
              className="h-0.5 bg-blue-500 rounded"
              style={{ width: `${episode.progress.percentComplete}%` }}
            />
          </div>
        )}
      </div>

      {episode.downloaded && (
        <span
          className="flex-shrink-0 mt-1 text-xs text-green-500 font-medium"
          title="Downloaded"
        >
          ↓
        </span>
      )}
      {!episode.downloaded && episode.progress === null && (
        <DownloadProgress episodeId={episode.id} />
      )}
    </div>
  );
}
