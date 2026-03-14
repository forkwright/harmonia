import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import type { BookmarkCreate } from "../../../types/media";

export function useBookmarks(audiobookId: string) {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();
  const queryKey = ["bookmarks", audiobookId, token];

  const query = useQuery({
    queryKey,
    queryFn: () => api.getBookmarks(audiobookId, token),
    enabled: token.length > 0 && audiobookId.length > 0,
  });

  const create = useMutation({
    mutationFn: (bookmark: BookmarkCreate) =>
      api.createBookmark(audiobookId, bookmark, token),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });

  const remove = useMutation({
    mutationFn: (bookmarkId: string) => api.deleteBookmark(bookmarkId, token),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });

  return {
    bookmarks: query.data?.data ?? [],
    isLoading: query.isLoading,
    isError: query.isError,
    create,
    remove,
  };
}
