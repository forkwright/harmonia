import { useCallback } from "react";
import { VirtuosoGrid } from "react-virtuoso";
import clsx from "clsx";
import { useAudiobooks } from "./hooks";
import { useLibraryStore } from "./store";
import type { Audiobook } from "../../types/api";

function formatDuration(ms: number | null): string {
  if (ms == null) return "";
  const totalMins = Math.floor(ms / 60_000);
  const hours = Math.floor(totalMins / 60);
  const mins = totalMins % 60;
  return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
}

function AudiobookCard({ book }: { book: Audiobook }) {
  const initial = book.title.charAt(0).toUpperCase();
  return (
    <div
      className={clsx(
        "bg-gray-800 rounded-lg overflow-hidden cursor-pointer",
        "hover:bg-gray-700 transition-colors group"
      )}
    >
      <div className="aspect-square bg-gray-700 flex items-center justify-center">
        <span className="text-4xl font-bold text-gray-500 group-hover:text-gray-400 select-none">
          {initial}
        </span>
      </div>
      <div className="p-3">
        <p className="text-sm font-medium text-white truncate" title={book.title}>
          {book.title}
        </p>
        {book.series_name && (
          <p className="text-xs text-gray-400 truncate mt-0.5">
            {book.series_name}
            {book.series_position != null ? ` #${book.series_position}` : ""}
          </p>
        )}
        {book.duration_ms != null && (
          <p className="text-xs text-gray-500 mt-1">{formatDuration(book.duration_ms)}</p>
        )}
      </div>
    </div>
  );
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function AudiobooksPage() {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, isError } =
    useAudiobooks();
  const token = useLibraryStore((s) => s.token);

  const books = data?.pages.flatMap((p) => p.data) ?? [];

  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  if (!token) return <EmptyState message="Set an API token in Settings to browse your library." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load audiobooks." />;
  if (books.length === 0) return <EmptyState message="No audiobooks found." />;

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-hidden">
        <VirtuosoGrid
          style={{ height: "100%" }}
          totalCount={books.length}
          endReached={endReached}
          itemContent={(index) => (
            <div className="p-2">
              <AudiobookCard book={books[index]} />
            </div>
          )}
          listClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6"
          components={{
            Footer: () =>
              isFetchingNextPage ? (
                <div className="py-4 text-center text-sm text-gray-500">Loading…</div>
              ) : null,
          }}
        />
      </div>
    </div>
  );
}
