import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import type { WantedQueryParams, WantedItemCreate } from "../../../types/management";

export function useWanted(params: WantedQueryParams = {}) {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const wanted = useQuery({
    queryKey: ["manage-wanted", token, params],
    queryFn: () => api.getWantedMedia(params, token),
    enabled: token.length > 0,
  });

  const add = useMutation({
    mutationFn: (item: WantedItemCreate) => api.addWanted(item, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-wanted"] }),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.removeWanted(id, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-wanted"] }),
  });

  const triggerSearch = useMutation({
    mutationFn: (id: string) => api.triggerWantedSearch(id, token),
  });

  return { wanted, add, remove, triggerSearch };
}
