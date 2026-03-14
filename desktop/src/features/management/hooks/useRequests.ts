import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import type { RequestQueryParams, RequestCreate } from "../../../types/management";

export function useRequests(params: RequestQueryParams = {}) {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const requests = useQuery({
    queryKey: ["manage-requests", token, params],
    queryFn: () => api.getRequests(params, token),
    enabled: token.length > 0,
  });

  const create = useMutation({
    mutationFn: (request: RequestCreate) => api.createRequest(request, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-requests"] }),
  });

  const approve = useMutation({
    mutationFn: (requestId: string) => api.approveRequest(requestId, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-requests"] }),
  });

  const deny = useMutation({
    mutationFn: ({ requestId, reason }: { requestId: string; reason: string }) =>
      api.denyRequest(requestId, reason, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-requests"] }),
  });

  const cancel = useMutation({
    mutationFn: (requestId: string) => api.cancelRequest(requestId, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-requests"] }),
  });

  return { requests, create, approve, deny, cancel };
}
