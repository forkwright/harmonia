import { useInfiniteQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import { useAudiobookLibraryStore } from "../store";

const PER_PAGE = 48;

export function useAudiobooks() {
  const token = useLibraryStore((s) => s.token);
  const filter = useAudiobookLibraryStore((s) => s.filter);
  const sort = useAudiobookLibraryStore((s) => s.sort);

  return useInfiniteQuery({
    queryKey: ["audiobooks-full", token, filter, sort],
    queryFn: ({ pageParam }) =>
      api.getAudiobooks({ page: pageParam as number, per_page: PER_PAGE, filter, sort }, token),
    initialPageParam: 1,
    getNextPageParam: (lastPage) => {
      if (lastPage.data.length < PER_PAGE) return undefined;
      return (lastPage.meta?.page ?? 1) + 1;
    },
    enabled: token.length > 0,
  });
}
