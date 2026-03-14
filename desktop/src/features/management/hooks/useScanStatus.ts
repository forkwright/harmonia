import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

const SCAN_POLL_INTERVAL = 3_000;

export function useScanStatus() {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const status = useQuery({
    queryKey: ["manage-scan-status", token],
    queryFn: () => api.getScanStatus(token),
    enabled: token.length > 0,
    refetchInterval: (query) => {
      const data = query.state.data;
      return data?.running ? SCAN_POLL_INTERVAL : false;
    },
  });

  const trigger = useMutation({
    mutationFn: (path?: string) => api.triggerLibraryScan(token, path),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-scan-status"] }),
  });

  return { status, trigger };
}
