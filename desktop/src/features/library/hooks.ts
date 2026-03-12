import { useInfiniteQuery } from "@tanstack/react-query";
import { api } from "../../api/client";
import { useLibraryStore } from "./store";

const PER_PAGE = 50;

export function useAlbums() {
  const token = useLibraryStore((s) => s.token);
  return useInfiniteQuery({
    queryKey: ["albums", token],
    queryFn: ({ pageParam }) =>
      api.listReleaseGroups({ page: pageParam as number, per_page: PER_PAGE }, token),
    initialPageParam: 1,
    getNextPageParam: (lastPage) => {
      if (lastPage.data.length < PER_PAGE) return undefined;
      return (lastPage.meta?.page ?? 1) + 1;
    },
    enabled: token.length > 0,
  });
}

export function useTracks() {
  const token = useLibraryStore((s) => s.token);
  return useInfiniteQuery({
    queryKey: ["tracks", token],
    queryFn: ({ pageParam }) =>
      api.listTracks({ page: pageParam as number, per_page: PER_PAGE }, token),
    initialPageParam: 1,
    getNextPageParam: (lastPage) => {
      if (lastPage.data.length < PER_PAGE) return undefined;
      return (lastPage.meta?.page ?? 1) + 1;
    },
    enabled: token.length > 0,
  });
}

export function useAudiobooks() {
  const token = useLibraryStore((s) => s.token);
  return useInfiniteQuery({
    queryKey: ["audiobooks", token],
    queryFn: ({ pageParam }) =>
      api.listAudiobooks({ page: pageParam as number, per_page: PER_PAGE }, token),
    initialPageParam: 1,
    getNextPageParam: (lastPage) => {
      if (lastPage.data.length < PER_PAGE) return undefined;
      return (lastPage.meta?.page ?? 1) + 1;
    },
    enabled: token.length > 0,
  });
}
