import { useCallback } from "react";
import { Virtuoso } from "react-virtuoso";
import { useTracks } from "./hooks";
import { useLibraryStore } from "./store";

function formatDuration(ms: number | null): string {
  if (ms == null) return "—";
  const totalSecs = Math.floor(ms / 1000);
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function TracksPage() {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, isError } = useTracks();
  const token = useLibraryStore((s) => s.token);

  const tracks = data?.pages.flatMap((p) => p.data) ?? [];

  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  if (!token) return <EmptyState message="Set an API token in Settings to browse your library." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load tracks." />;
  if (tracks.length === 0) return <EmptyState message="No tracks found." />;

  return (
    <div className="h-full">
      <Virtuoso
        style={{ height: "100%" }}
        totalCount={tracks.length}
        endReached={endReached}
        itemContent={(index) => {
          const track = tracks[index];
          return (
            <div className="flex items-center gap-4 px-6 py-3 hover:bg-gray-800 transition-colors border-b border-gray-800/50">
              <span className="text-xs text-gray-600 w-6 text-right tabular-nums select-none">
                {track.position}
              </span>
              <span className="flex-1 text-sm text-white truncate">{track.title}</span>
              {track.codec && (
                <span className="text-xs text-blue-400 font-mono bg-blue-900/30 px-1.5 py-0.5 rounded">
                  {track.codec.toUpperCase()}
                </span>
              )}
              <span className="text-xs text-gray-500 w-12 text-right tabular-nums">
                {formatDuration(track.duration_ms)}
              </span>
            </div>
          );
        }}
        components={{
          Footer: () =>
            isFetchingNextPage ? (
              <div className="py-4 text-center text-sm text-gray-500">Loading…</div>
            ) : null,
        }}
      />
    </div>
  );
}
