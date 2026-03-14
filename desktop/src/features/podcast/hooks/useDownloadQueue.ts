/** TanStack Query hooks for podcast download queue management. */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

const QUEUE_POLL_INTERVAL_MS = 5_000;

export function useDownloadQueue() {
  const token = useLibraryStore((s) => s.token);
  return useQuery({
    queryKey: ["podcasts", "downloads", token],
    queryFn: () => api.getDownloadQueue(token),
    enabled: token.length > 0,
    // WHY: Poll while downloads are active to show live progress.
    refetchInterval: QUEUE_POLL_INTERVAL_MS,
  });
}

export function useDownloadEpisode() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (episodeId: string) => api.downloadEpisode(episodeId, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "downloads"] });
    },
  });
}

export function useCancelDownload() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (episodeId: string) => api.cancelDownload(episodeId, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "downloads"] });
    },
  });
}

export function useDeleteDownload() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (episodeId: string) => api.deleteDownload(episodeId, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "episodes"] });
    },
  });
}
