/** Tracks per-episode playback position and syncs to the server. */

import { useEffect, useRef } from "react";
import { useLibraryStore } from "../../library/store";
import { usePodcastStore } from "../store";
import { api } from "../../../api/client";

const SYNC_INTERVAL_MS = 30_000;
// WHY: 95% matches industry convention for marking podcasts auto-complete.
const COMPLETION_THRESHOLD = 0.95;

export function useEpisodeProgress(episodeId: string | null, durationMs: number) {
  const token = useLibraryStore((s) => s.token);
  const isPlaying = usePodcastStore((s) => s.isPlaying);
  const positionMs = usePodcastStore((s) => s.positionMs);
  const positionRef = useRef(positionMs);

  // Keep ref current so the sync interval always uses the latest position.
  useEffect(() => {
    positionRef.current = positionMs;
  });

  // Periodic server sync during playback.
  useEffect(() => {
    if (!episodeId || !isPlaying || !token) return;

    const id = setInterval(() => {
      void api.updateEpisodeProgress(episodeId, { positionMs: positionRef.current }, token);
    }, SYNC_INTERVAL_MS);

    return () => clearInterval(id);
  }, [episodeId, isPlaying, token]);

  // Sync immediately on pause.
  useEffect(() => {
    if (!episodeId || isPlaying || !token || positionMs === 0) return;
    void api.updateEpisodeProgress(episodeId, { positionMs }, token);
  }, [episodeId, isPlaying, positionMs, token]);

  // Auto-complete at 95% of episode duration.
  useEffect(() => {
    if (!episodeId || !token || durationMs === 0) return;
    if (positionMs / durationMs >= COMPLETION_THRESHOLD) {
      void api.markEpisodeCompleted(episodeId, token);
    }
  }, [episodeId, positionMs, durationMs, token]);
}
