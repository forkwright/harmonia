import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import type { SearchQuery, DownloadRequest } from "../../../types/management";

export function useIndexerSearch() {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const search = useMutation({
    mutationFn: (query: SearchQuery) => api.searchIndexers(query, token),
  });

  const grab = useMutation({
    mutationFn: (req: DownloadRequest) => api.triggerDownload(req, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-download-queue"] }),
  });

  return { search, grab };
}
