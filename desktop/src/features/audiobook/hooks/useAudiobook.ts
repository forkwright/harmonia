import { useQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

export function useAudiobook(id: string) {
  const token = useLibraryStore((s) => s.token);
  return useQuery({
    queryKey: ["audiobook", id, token],
    queryFn: () => api.getAudiobook(id, token),
    enabled: token.length > 0 && id.length > 0,
  });
}

export function useAudiobookProgress(id: string) {
  const token = useLibraryStore((s) => s.token);
  return useQuery({
    queryKey: ["audiobook-progress", id, token],
    queryFn: () => api.getAudiobookProgress(id, token),
    enabled: token.length > 0 && id.length > 0,
    staleTime: 0,
  });
}
