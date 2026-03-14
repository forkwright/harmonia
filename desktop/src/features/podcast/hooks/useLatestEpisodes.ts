/** TanStack Query hook for the cross-podcast latest episodes feed. */

import { useInfiniteQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

const PAGE_SIZE = 50;
const DEFAULT_HOURS_BACK = 168; // 1 week

export function useLatestEpisodes(hoursBack: number = DEFAULT_HOURS_BACK) {
  const token = useLibraryStore((s) => s.token);
  return useInfiniteQuery({
    queryKey: ["podcasts", "latest", hoursBack, token],
    queryFn: ({ pageParam }) =>
      api.getLatestEpisodes(
        { page: pageParam as number, pageSize: PAGE_SIZE, hoursBack },
        token,
      ),
    initialPageParam: 1,
    getNextPageParam: (lastPage) => {
      if (lastPage.data.length < PAGE_SIZE) return undefined;
      return (lastPage.meta?.page ?? 1) + 1;
    },
    enabled: token.length > 0,
  });
}
