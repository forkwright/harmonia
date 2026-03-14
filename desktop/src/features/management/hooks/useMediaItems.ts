import { useQuery } from "@tanstack/react-query";
import { api } from "../../../api/client";
import { useLibraryStore } from "../../library/store";
import { useManagementStore } from "../store";
import type { MediaQueryParams } from "../../../types/management";

export function useMediaItems(overrides?: Partial<MediaQueryParams>) {
  const token = useLibraryStore((s) => s.token);
  const selectedMediaType = useManagementStore((s) => s.selectedMediaType);
  const filters = useManagementStore((s) => s.filters);

  const params: MediaQueryParams = {
    mediaType: selectedMediaType,
    status: filters.status !== "all" ? filters.status : undefined,
    qualityTier: filters.qualityTier !== "all" ? filters.qualityTier : undefined,
    hasMetadata:
      filters.hasMetadata === "yes" ? true : filters.hasMetadata === "no" ? false : undefined,
    page: 1,
    pageSize: 50,
    ...overrides,
  };

  return useQuery({
    queryKey: ["manage-media-items", token, params],
    queryFn: () => api.getMediaItems(params, token),
    enabled: token.length > 0,
  });
}
