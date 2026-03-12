import { useMemo, useCallback } from "react";
import { VirtuosoGrid } from "react-virtuoso";
import { useAlbums } from "./hooks";
import AlbumCard from "./AlbumCard";
import SortFilterBar from "./SortFilterBar";
import { useLibraryStore } from "./store";
import type { ReleaseGroup } from "../../types/api";

function sortAlbums(albums: ReleaseGroup[], sort: string): ReleaseGroup[] {
  return [...albums].sort((a, b) => {
    if (sort === "year") return (b.year ?? 0) - (a.year ?? 0);
    if (sort === "added") return b.added_at.localeCompare(a.added_at);
    return a.title.localeCompare(b.title);
  });
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function AlbumsPage() {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, isError } = useAlbums();
  const token = useLibraryStore((s) => s.token);
  const sort = useLibraryStore((s) => s.sort);

  const albums = useMemo(() => {
    const flat = data?.pages.flatMap((p) => p.data) ?? [];
    return sortAlbums(flat, sort);
  }, [data, sort]);

  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  if (!token) return <EmptyState message="Set an API token in Settings to browse your library." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load albums." />;
  if (albums.length === 0) return <EmptyState message="No albums found." />;

  return (
    <div className="flex flex-col h-full">
      <SortFilterBar />
      <div className="flex-1 overflow-hidden">
        <VirtuosoGrid
          style={{ height: "100%" }}
          totalCount={albums.length}
          endReached={endReached}
          itemContent={(index) => (
            <div className="p-2">
              <AlbumCard album={albums[index]} />
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
