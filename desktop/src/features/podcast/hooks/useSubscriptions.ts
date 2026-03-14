/** TanStack Query hooks for podcast subscription management. */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

export function useSubscriptions() {
  const token = useLibraryStore((s) => s.token);
  return useQuery({
    queryKey: ["podcasts", "subscriptions", token],
    queryFn: () => api.getSubscriptions(token),
    enabled: token.length > 0,
  });
}

export function useSubscribe() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (feedUrl: string) => api.subscribe(feedUrl, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "subscriptions"] });
    },
  });
}

export function useUnsubscribe() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (podcastId: string) => api.unsubscribe(podcastId, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "subscriptions"] });
    },
  });
}

export function useRefreshFeed() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: (podcastId: string) => api.refreshFeed(podcastId, token),
    onSuccess: (_data, podcastId) => {
      void client.invalidateQueries({ queryKey: ["podcasts", "episodes", podcastId] });
    },
  });
}

export function useRefreshAllFeeds() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => api.refreshAllFeeds(token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts"] });
    },
  });
}

export function useUpdateSubscription() {
  const token = useLibraryStore((s) => s.token);
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({
      podcastId,
      settings,
    }: {
      podcastId: string;
      settings: { autoDownload?: boolean; refreshIntervalMinutes?: number };
    }) => api.updateSubscription(podcastId, settings, token),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["podcasts", "subscriptions"] });
    },
  });
}
