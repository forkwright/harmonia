/** TanStack Query hook for a single podcast's metadata. */

import { useQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

export function usePodcast(podcastId: string) {
  const token = useLibraryStore((s) => s.token);
  return useQuery({
    queryKey: ["podcasts", "subscription", podcastId, token],
    queryFn: () => api.getSubscriptions(token).then((subs) => subs.find((s) => s.id === podcastId)),
    enabled: token.length > 0 && podcastId.length > 0,
  });
}
