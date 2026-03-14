import { useMemo, useCallback } from "react";
import { VirtuosoGrid } from "react-virtuoso";
import { invoke } from "@tauri-apps/api/core";
import { useAlbums } from "./hooks";
import AlbumCard from "./AlbumCard";
import SortFilterBar from "./SortFilterBar";
import { useLibraryStore } from "./store";
import { api } from "../../api/client";
import { usePlayback } from "../now-playing/hooks/usePlayback";
import type { ReleaseGroup, Track } from "../../types/api";
import type { QueueEntry } from "../../types/playback";

function sortAlbums(albums: ReleaseGroup[], sort: string): ReleaseGroup[] {
  return [...albums].sort((a, b) => {
    if (sort === "year") return (b.year ?? 0) - (a.year ?? 0);
    if (sort === "added") return b.added_at.localeCompare(a.added_at);
    return a.title.localeCompare(b.title);
  });
}

function trackToEntry(track: Track, albumTitle?: string): QueueEntry {
  return {
    track_id: track.id,
    title: track.title,
    artist: null,
    album: albumTitle ?? null,
    duration_ms: track.duration_ms,
  };
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

interface AlbumActionsProps {
  album: ReleaseGroup;
  onPlay: (album: ReleaseGroup, shuffle: boolean) => void;
  onAddToQueue: (album: ReleaseGroup) => void;
}

function AlbumActions({ album, onPlay, onAddToQueue }: AlbumActionsProps) {
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-1 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg">
      <button
        className="text-xs text-white bg-white/20 hover:bg-white/30 px-3 py-1.5 rounded-full transition-colors w-28"
        title="Play album"
        onClick={(e) => {
          e.stopPropagation();
          onPlay(album, false);
        }}
      >
        ▶ Play
      </button>
      <button
        className="text-xs text-gray-200 bg-white/10 hover:bg-white/20 px-3 py-1.5 rounded-full transition-colors w-28"
        title="Shuffle album"
        onClick={(e) => {
          e.stopPropagation();
          onPlay(album, true);
        }}
      >
        ⇌ Shuffle
      </button>
      <button
        className="text-xs text-gray-200 bg-white/10 hover:bg-white/20 px-3 py-1.5 rounded-full transition-colors w-28"
        title="Add album to queue"
        onClick={(e) => {
          e.stopPropagation();
          onAddToQueue(album);
        }}
      >
        + Add to Queue
      </button>
    </div>
  );
}

export default function AlbumsPage() {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, isError } = useAlbums();
  const token = useLibraryStore((s) => s.token);
  const sort = useLibraryStore((s) => s.sort);
  const { playTrack, queueAdd, setShuffle } = usePlayback();

  const albums = useMemo(() => {
    const flat = data?.pages.flatMap((p) => p.data) ?? [];
    return sortAlbums(flat, sort);
  }, [data, sort]);

  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  const fetchAlbumTracks = useCallback(
    async (album: ReleaseGroup): Promise<Track[]> => {
      try {
        const response = await api.listTracksForAlbum(album.id, token);
        return response.data;
      } catch {
        return [];
      }
    },
    [token]
  );

  const handlePlayAlbum = useCallback(
    async (album: ReleaseGroup, shuffle: boolean) => {
      const tracks = await fetchAlbumTracks(album);
      if (tracks.length === 0) return;
      const entries: QueueEntry[] = tracks.map((t) => trackToEntry(t, album.title));
      const baseUrl = await invoke<string>("get_server_url");
      if (shuffle) {
        await setShuffle(true);
      }
      await playTrack(entries[0], baseUrl, token || undefined);
      if (entries.length > 1) {
        await queueAdd(entries.slice(1));
      }
    },
    [fetchAlbumTracks, playTrack, queueAdd, setShuffle, token]
  );

  const handleAddAlbumToQueue = useCallback(
    async (album: ReleaseGroup) => {
      const tracks = await fetchAlbumTracks(album);
      if (tracks.length === 0) return;
      const entries: QueueEntry[] = tracks.map((t) => trackToEntry(t, album.title));
      await queueAdd(entries);
    },
    [fetchAlbumTracks, queueAdd]
  );

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
              <div className="relative group">
                <AlbumCard album={albums[index]} />
                <AlbumActions
                  album={albums[index]}
                  onPlay={(album, shuffle) => {
                    void handlePlayAlbum(album, shuffle);
                  }}
                  onAddToQueue={(album) => {
                    void handleAddAlbumToQueue(album);
                  }}
                />
              </div>
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
