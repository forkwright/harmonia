import { useQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

export function useMediaItem(id: string) {
  const token = useLibraryStore((s) => s.token);

  return useQuery({
    queryKey: ["manage-media-item", id, token],
    queryFn: () => api.getMediaItem(id, token),
    enabled: token.length > 0 && id.length > 0,
  });
}
