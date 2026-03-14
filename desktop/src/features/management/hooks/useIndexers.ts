import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import type { IndexerCreate, IndexerUpdate } from "../../../types/management";

export function useIndexers() {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const indexers = useQuery({
    queryKey: ["manage-indexers", token],
    queryFn: () => api.getIndexers(token),
    enabled: token.length > 0,
  });

  const add = useMutation({
    mutationFn: (config: IndexerCreate) => api.addIndexer(config, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-indexers"] }),
  });

  const update = useMutation({
    mutationFn: ({ id, config }: { id: string; config: IndexerUpdate }) =>
      api.updateIndexer(id, config, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-indexers"] }),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteIndexer(id, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-indexers"] }),
  });

  const test = useMutation({
    mutationFn: (id: string) => api.testIndexer(id, token),
  });

  return { indexers, add, update, remove, test };
}
