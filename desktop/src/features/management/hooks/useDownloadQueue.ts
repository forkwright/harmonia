import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

const QUEUE_POLL_INTERVAL = 5_000;

export function useDownloadQueue() {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const queue = useQuery({
    queryKey: ["manage-download-queue", token],
    queryFn: () => api.getManageDownloadQueue(token),
    enabled: token.length > 0,
    refetchInterval: QUEUE_POLL_INTERVAL,
  });

  const cancel = useMutation({
    mutationFn: (downloadId: string) => api.cancelManageDownload(downloadId, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-download-queue"] }),
  });

  const retry = useMutation({
    mutationFn: (downloadId: string) => api.retryDownload(downloadId, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-download-queue"] }),
  });

  return { queue, cancel, retry };
}
