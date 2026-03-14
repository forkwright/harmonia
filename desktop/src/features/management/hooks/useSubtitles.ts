import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

export function useSubtitles(mediaId: string) {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const subtitles = useQuery({
    queryKey: ["manage-subtitles", mediaId, token],
    queryFn: () => api.getSubtitles(mediaId, token),
    enabled: token.length > 0 && mediaId.length > 0,
  });

  const search = useQuery({
    queryKey: ["manage-subtitle-search", mediaId, token],
    queryFn: () => api.searchSubtitles(mediaId, token),
    enabled: false,
  });

  const download = useMutation({
    mutationFn: (subtitleId: string) => api.downloadSubtitle(mediaId, subtitleId, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-subtitles", mediaId] }),
  });

  const remove = useMutation({
    mutationFn: (subtitleId: string) => api.deleteSubtitle(subtitleId, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-subtitles", mediaId] }),
  });

  return { subtitles, search, download, remove };
}
