/** TanStack Query hooks for episode listing and mutations. */

import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import type { EpisodeQueryParams } from "../../../types/media";

const PAGE_SIZE = 50;

export function useEpisodes(podcastId: string, filter: EpisodeQueryParams["filter"] = "all") {
  const token = useLibraryStore((s) => s.token);
  return useInfiniteQuery({
    queryKey: ["podcasts", "episodes", podcastId, filter, token],
    queryFn: ({ pageParam }) =>
      api.getEpisodes(
        podcastId,
        { page: pageParam as number, pageSize: PAGE_SIZE, filter },
        token,
      ),
    initialPageParam: 1,
    getNextPageParam: (lastPage) => {
      if (lastPage.data.length < PAGE_SIZE) return undefined;
      return (lastPage.meta?.page ?? 1) + 1;
    },
    enabled: token.length > 0 && podcastId.length > 0,
  });
}

export function useMarkEpisodeCompleted() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (episodeId: string) => api.markEpisodeCompleted(episodeId, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "episodes"] });
    },
  });
}

export function useMarkEpisodeUnplayed() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (episodeId: string) => api.markEpisodeUnplayed(episodeId, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "episodes"] });
    },
  });
}
