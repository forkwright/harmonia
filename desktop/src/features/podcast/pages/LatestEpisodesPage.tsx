/** Cross-podcast latest episodes feed, grouped by date. */

import { useMemo, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useLatestEpisodes } from "../hooks/useLatestEpisodes";
import { usePodcastPlayback } from "../hooks/usePodcastPlayback";
import { useLibraryStore } from "../../library/store";
import { Virtuoso } from "react-virtuoso";
import type { Episode } from "../../../types/media";

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function dayLabel(iso: string): string {
  const date = new Date(iso);
  const now = new Date();
  const diff = Math.floor((now.getTime() - date.getTime()) / 86_400_000);
  if (diff === 0) return "Today";
  if (diff === 1) return "Yesterday";
  if (diff < 7) return "This Week";
  return "Older";
}

type Row = { type: "header"; label: string } | { type: "episode"; episode: Episode };

function groupEpisodes(episodes: Episode[]): Row[] {
  const rows: Row[] = [];
  let lastLabel = "";
  for (const ep of episodes) {
    const label = dayLabel(ep.publishedAt);
    if (label !== lastLabel) {
      rows.push({ type: "header", label });
      lastLabel = label;
    }
    rows.push({ type: "episode", episode: ep });
  }
  return rows;
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function LatestEpisodesPage() {
  const token = useLibraryStore((s) => s.token);
  const { data, isLoading, isError, fetchNextPage, hasNextPage, isFetchingNextPage } =
    useLatestEpisodes();
  const { playEpisode } = usePodcastPlayback();
  const navigate = useNavigate();

  const episodes = useMemo(() => data?.pages.flatMap((p) => p.data) ?? [], [data]);
  const rows = useMemo(() => groupEpisodes(episodes), [episodes]);

  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  if (!token) return <EmptyState message="Set an API token in Settings to browse podcasts." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load latest episodes." />;
  if (rows.length === 0) return <EmptyState message="No recent episodes found." />;

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 py-3 border-b border-gray-800 flex-shrink-0">
        <h1 className="text-sm font-semibold text-gray-100">Latest Episodes</h1>
      </div>
      <div className="flex-1 overflow-hidden">
        <Virtuoso
          style={{ height: "100%" }}
          data={rows}
          endReached={endReached}
          itemContent={(_index, row) => {
            if (row.type === "header") {
              return (
                <div className="px-4 py-2 text-xs font-semibold text-gray-500 uppercase tracking-wider bg-gray-900/80">
                  {row.label}
                </div>
              );
            }
            const ep = row.episode;
            return (
              <div className="flex items-center gap-3 px-4 py-3 border-b border-gray-800 hover:bg-gray-800/50 transition-colors">
                <button
                  onClick={() => void playEpisode(ep.id)}
                  className="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 hover:bg-blue-500 flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-blue-400"
                  aria-label={`Play ${ep.title}`}
                >
                  <span className="text-white text-[10px] ml-0.5">▶</span>
                </button>
                <div className="flex-1 min-w-0">
                  <button
                    onClick={() => navigate(`/library/podcasts/episodes/${ep.id}`)}
                    className="text-sm font-medium text-gray-100 truncate block text-left hover:underline w-full"
                  >
                    {ep.title}
                  </button>
                  <p className="text-xs text-gray-500 truncate">{ep.podcastTitle}</p>
                </div>
                <span className="flex-shrink-0 text-xs text-gray-500">
                  {formatDuration(ep.durationMs)}
                </span>
                {ep.progress?.completed && (
                  <span className="flex-shrink-0 text-xs text-green-500">Played</span>
                )}
              </div>
            );
          }}
        />
      </div>
    </div>
  );
}
