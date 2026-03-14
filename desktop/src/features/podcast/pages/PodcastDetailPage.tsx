/** Single podcast: header, subscription controls, filtered episode list. */

import { useState, useMemo, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { usePodcast } from "../hooks/usePodcast";
import { useEpisodes, useMarkEpisodeCompleted } from "../hooks/useEpisodes";
import { useUnsubscribe, useUpdateSubscription } from "../hooks/useSubscriptions";
import { usePodcastPlayback } from "../hooks/usePodcastPlayback";
import EpisodeList from "../components/EpisodeList";
import type { EpisodeQueryParams } from "../../../types/media";

const REFRESH_INTERVALS = [15, 30, 60, 120, 360, 720, 1440] as const;

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function PodcastDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const podcastId = id ?? "";

  const { data: podcast, isLoading, isError } = usePodcast(podcastId);
  const [filter, setFilter] = useState<EpisodeQueryParams["filter"]>("all");
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage } = useEpisodes(podcastId, filter);
  const unsubscribe = useUnsubscribe();
  const updateSubscription = useUpdateSubscription();
  const markAllPlayed = useMarkEpisodeCompleted();
  const { playEpisode, resumeEpisode } = usePodcastPlayback();

  const episodes = useMemo(() => data?.pages.flatMap((p) => p.data) ?? [], [data]);

  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  const handlePlay = useCallback(
    (episodeId: string) => {
      const ep = episodes.find((e) => e.id === episodeId);
      if (!ep) return;
      const pos = ep.progress?.positionMs ?? 0;
      if (pos > 0) {
        void resumeEpisode(episodeId, pos);
      } else {
        void playEpisode(episodeId);
      }
    },
    [episodes, playEpisode, resumeEpisode],
  );

  const handleUnsubscribe = () => {
    unsubscribe.mutate(podcastId, {
      onSuccess: () => navigate("/library/podcasts"),
    });
  };

  const handleAutoDownloadToggle = () => {
    if (!podcast) return;
    updateSubscription.mutate({
      podcastId,
      settings: { autoDownload: !podcast.autoDownload },
    });
  };

  const handleRefreshIntervalChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    updateSubscription.mutate({
      podcastId,
      settings: { refreshIntervalMinutes: Number(e.target.value) },
    });
  };

  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError || !podcast) return <EmptyState message="Podcast not found." />;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-start gap-4 px-6 py-5 border-b border-gray-800 flex-shrink-0">
        <div className="w-24 h-24 rounded-lg overflow-hidden bg-gray-700 flex-shrink-0">
          {podcast.imageUrl ? (
            <img src={podcast.imageUrl} alt={podcast.title} className="w-full h-full object-cover" />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-4xl">🎙</div>
          )}
        </div>
        <div className="flex-1 min-w-0">
          <h1 className="text-lg font-semibold text-gray-100">{podcast.title}</h1>
          {podcast.author && <p className="text-sm text-gray-400 mt-0.5">{podcast.author}</p>}
          {podcast.description && (
            <p className="text-xs text-gray-500 mt-2 line-clamp-2">{podcast.description}</p>
          )}
          <div className="flex items-center gap-3 mt-3 flex-wrap">
            <button
              onClick={handleUnsubscribe}
              disabled={unsubscribe.isPending}
              className="px-3 py-1.5 rounded-lg text-xs text-gray-300 bg-gray-800 hover:bg-gray-700 disabled:opacity-50 transition-colors"
            >
              Unsubscribe
            </button>
            <button
              onClick={() => {
                episodes.forEach((ep) => {
                  if (!ep.progress?.completed) markAllPlayed.mutate(ep.id);
                });
              }}
              className="px-3 py-1.5 rounded-lg text-xs text-gray-300 bg-gray-800 hover:bg-gray-700 transition-colors"
            >
              Mark all played
            </button>
          </div>

          {/* Subscription settings */}
          <div className="flex items-center gap-4 mt-3 flex-wrap">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={podcast.autoDownload}
                onChange={handleAutoDownloadToggle}
                disabled={updateSubscription.isPending}
                className="rounded accent-blue-500"
              />
              <span className="text-xs text-gray-400">Auto-download</span>
            </label>

            <div className="flex items-center gap-1.5">
              <label htmlFor="refresh-interval" className="text-xs text-gray-400">
                Refresh every
              </label>
              <select
                id="refresh-interval"
                value={podcast.refreshIntervalMinutes}
                onChange={handleRefreshIntervalChange}
                disabled={updateSubscription.isPending}
                className="text-xs bg-gray-800 text-gray-300 border border-gray-700 rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-blue-500"
              >
                {REFRESH_INTERVALS.map((m) => (
                  <option key={m} value={m}>
                    {m < 60 ? `${m}m` : `${m / 60}h`}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>
      </div>

      {/* Filter bar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-gray-800 flex-shrink-0">
        {(["all", "unplayed", "in_progress", "completed", "downloaded"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
              filter === f
                ? "bg-blue-600 text-white"
                : "text-gray-400 hover:text-gray-200 hover:bg-gray-800"
            }`}
          >
            {f === "in_progress" ? "In progress" : f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
      </div>

      {/* Episode list */}
      <div className="flex-1 overflow-hidden">
        {episodes.length === 0 ? (
          <EmptyState message="No episodes found for this filter." />
        ) : (
          <EpisodeList episodes={episodes} onPlay={handlePlay} onEndReached={endReached} />
        )}
      </div>
    </div>
  );
}
