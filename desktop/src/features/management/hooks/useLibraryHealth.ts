import { useQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";

export function useLibraryHealth() {
  const token = useLibraryStore((s) => s.token);

  return useQuery({
    queryKey: ["manage-library-health", token],
    queryFn: () => api.getLibraryHealth(token),
    enabled: token.length > 0,
  });
}
