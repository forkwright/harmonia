import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

interface PlaybackPosition {
  audiobookId: string;
  chapterIndex: number;
  chapterOffsetMs: number;
  chapterTitle: string;
  playbackSpeed: number;
  isPlaying: boolean;
}

const POLL_INTERVAL_MS = 1000;
const POSITION_QUERY_KEY = ["audiobook-position"] as const;

async function fetchPosition(): Promise<PlaybackPosition | null> {
  return invoke<PlaybackPosition | null>("audiobook_get_position");
}

export function usePositionSync() {
  const { data: position } = useQuery({
    queryKey: POSITION_QUERY_KEY,
    queryFn: fetchPosition,
    refetchInterval: POLL_INTERVAL_MS,
  });

  return { position: position ?? null };
}
