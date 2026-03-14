/** Single episode: metadata, play/resume, show notes, and actions. */

import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import { usePodcastPlayback } from "../hooks/usePodcastPlayback";
import { useEpisodeProgress } from "../hooks/useEpisodeProgress";
import ShowNotesPanel from "../components/ShowNotesPanel";
import EpisodeActions from "../components/EpisodeActions";

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1_000_000) return `${Math.round(bytes / 1_000)} KB`;
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function EpisodeDetailPage() {
  const { id } = useParams<{ id: string }>();
  const episodeId = id ?? "";
  const token = useLibraryStore((s) => s.token);

  const { data: episode, isLoading, isError } = useQuery({
    queryKey: ["podcasts", "episode", episodeId, token],
    queryFn: () => api.getEpisode(episodeId, token),
    enabled: token.length > 0 && episodeId.length > 0,
  });

  const { playEpisode, resumeEpisode } = usePodcastPlayback();

  // Register progress tracking for this episode.
  useEpisodeProgress(episode?.id ?? null, episode?.durationMs ?? 0);

  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError || !episode) return <EmptyState message="Episode not found." />;

  const resumePosition = episode.progress?.positionMs ?? 0;
  const isInProgress = resumePosition > 0 && !episode.progress?.completed;

  const handlePlay = () => {
    if (isInProgress) {
      void resumeEpisode(episode.id, resumePosition);
    } else {
      void playEpisode(episode.id);
    }
  };

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="px-6 py-6 max-w-3xl">
        <h1 className="text-xl font-semibold text-gray-100">{episode.title}</h1>
        <p className="text-sm text-gray-400 mt-1">{episode.podcastTitle}</p>

        {/* Metadata */}
        <div className="flex items-center gap-3 mt-2 text-xs text-gray-500">
          <span>{new Date(episode.publishedAt).toLocaleDateString()}</span>
          <span>·</span>
          <span>{formatDuration(episode.durationMs)}</span>
          {episode.fileSize !== null && (
            <>
              <span>·</span>
              <span>{formatBytes(episode.fileSize)}</span>
            </>
          )}
          <span>·</span>
          <span>{episode.enclosureType}</span>
        </div>

        {/* Play / Resume button */}
        <button
          onClick={handlePlay}
          className="mt-5 px-6 py-3 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-sm transition-colors focus:outline-none focus:ring-2 focus:ring-blue-400"
        >
          {isInProgress
            ? `Resume from ${Math.round(episode.progress!.percentComplete)}%`
            : "Play episode"}
        </button>

        {/* Actions */}
        <div className="mt-4">
          <EpisodeActions episode={episode} />
        </div>

        {/* Show notes */}
        <div className="mt-8">
          <h2 className="text-sm font-semibold text-gray-300 mb-3">Show notes</h2>
          <ShowNotesPanel html={episode.showNotes} />
        </div>
      </div>
    </div>
  );
}
