import { useState } from "react";
import { useQueryClient, useMutation } from "@tanstack/react-query";
import { useMediaItems } from "../hooks/useMediaItems";
import { useManagementStore } from "../store";
import { useLibraryStore } from "../../library/store";
import MediaTypeSelector from "../components/MediaTypeSelector";
import MediaItemRow from "../components/MediaItemRow";
import { api } from "../../../api/client";
import type { MediaType } from "../../../types/management";

const STATUS_OPTIONS = ["all", "imported", "enriched", "organized", "available", "failed"];
const QUALITY_OPTIONS = ["all", "lossless", "high", "good", "low"];

const MEDIA_TYPE_COUNTS: Partial<Record<MediaType, number>> = {};

export default function MediaBrowsePage() {
  const token = useLibraryStore((s) => s.token);
  const { filters, setFilter } = useManagementStore();
  const queryClient = useQueryClient();

  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [sortBy, setSortBy] = useState<"title" | "status" | "quality" | "size" | "addedAt">(
    "title",
  );
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("asc");
  const [bulkQualityProfile, setBulkQualityProfile] = useState("");

  const { data, isLoading, isError } = useMediaItems({ sortBy, sortOrder });

  const bulkRefresh = useMutation({
    mutationFn: () => api.bulkRefreshMetadata(selectedIds, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-media-items"] }),
  });

  const bulkDelete = useMutation({
    mutationFn: () => api.bulkDelete(selectedIds, token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["manage-media-items"] });
      setSelectedIds([]);
    },
  });

  const bulkSetProfile = useMutation({
    mutationFn: () => api.bulkSetQualityProfile(selectedIds, bulkQualityProfile, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-media-items"] }),
  });

  function toggleSelect(id: string, checked: boolean) {
    setSelectedIds((prev) => (checked ? [...prev, id] : prev.filter((x) => x !== id)));
  }

  function toggleAll(checked: boolean) {
    if (!data) return;
    setSelectedIds(checked ? data.data.map((item) => item.id) : []);
  }

  function toggleSort(col: typeof sortBy) {
    if (sortBy === col) {
      setSortOrder((o) => (o === "asc" ? "desc" : "asc"));
    } else {
      setSortBy(col);
      setSortOrder("asc");
    }
  }

  function sortIcon(col: typeof sortBy): string {
    if (sortBy !== col) return "↕";
    return sortOrder === "asc" ? "↑" : "↓";
  }

  const items = data?.data ?? [];
  const allSelected = items.length > 0 && selectedIds.length === items.length;

  return (
    <div className="h-full flex flex-col">
      <div className="border-b border-gray-800">
        <MediaTypeSelector counts={MEDIA_TYPE_COUNTS} />
      </div>

      <div className="flex gap-4 px-4 py-3 border-b border-gray-800 flex-wrap">
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-400">Status</label>
          <select
            value={filters.status}
            onChange={(e) => setFilter({ status: e.target.value })}
            className="text-xs rounded bg-gray-700 border border-gray-600 text-gray-300 px-2 py-1 focus:outline-none"
          >
            {STATUS_OPTIONS.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-400">Quality</label>
          <select
            value={filters.qualityTier}
            onChange={(e) => setFilter({ qualityTier: e.target.value })}
            className="text-xs rounded bg-gray-700 border border-gray-600 text-gray-300 px-2 py-1 focus:outline-none"
          >
            {QUALITY_OPTIONS.map((q) => (
              <option key={q} value={q}>
                {q}
              </option>
            ))}
          </select>
        </div>
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-400">Metadata</label>
          <select
            value={filters.hasMetadata}
            onChange={(e) =>
              setFilter({ hasMetadata: e.target.value as "all" | "yes" | "no" })
            }
            className="text-xs rounded bg-gray-700 border border-gray-600 text-gray-300 px-2 py-1 focus:outline-none"
          >
            <option value="all">All</option>
            <option value="yes">Has metadata</option>
            <option value="no">Missing metadata</option>
          </select>
        </div>
      </div>

      {selectedIds.length > 0 && (
        <div className="flex gap-3 px-4 py-2 bg-blue-900/20 border-b border-blue-800 flex-wrap">
          <span className="text-xs text-blue-300">{selectedIds.length} selected</span>
          <button
            onClick={() => bulkRefresh.mutate()}
            disabled={bulkRefresh.isPending}
            className="px-2 py-0.5 text-xs rounded bg-blue-700 hover:bg-blue-600 disabled:opacity-50 text-white transition-colors"
          >
            Refresh Metadata
          </button>
          <div className="flex gap-1">
            <input
              type="text"
              value={bulkQualityProfile}
              onChange={(e) => setBulkQualityProfile(e.target.value)}
              placeholder="Profile ID"
              className="px-2 py-0.5 text-xs rounded bg-gray-700 border border-gray-600 text-gray-300 focus:outline-none w-32"
            />
            <button
              onClick={() => bulkSetProfile.mutate()}
              disabled={bulkSetProfile.isPending || !bulkQualityProfile}
              className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 transition-colors"
            >
              Set Profile
            </button>
          </div>
          <button
            onClick={() => {
              if (confirm(`Delete ${selectedIds.length} items?`)) {
                bulkDelete.mutate();
              }
            }}
            disabled={bulkDelete.isPending}
            className="px-2 py-0.5 text-xs rounded bg-red-800 hover:bg-red-700 disabled:opacity-50 text-red-300 transition-colors"
          >
            Delete
          </button>
        </div>
      )}

      <div className="flex-1 overflow-auto">
        {isLoading && (
          <p className="p-4 text-sm text-gray-500">Loading…</p>
        )}
        {isError && (
          <p className="p-4 text-sm text-red-400">Failed to load media items.</p>
        )}
        {!isLoading && !isError && items.length === 0 && (
          <p className="p-4 text-sm text-gray-500">No items found.</p>
        )}
        {items.length > 0 && (
          <table className="w-full text-left">
            <thead className="sticky top-0 bg-gray-900 border-b border-gray-700">
              <tr>
                <th className="px-3 py-2 w-8">
                  <input
                    type="checkbox"
                    checked={allSelected}
                    onChange={(e) => toggleAll(e.target.checked)}
                    className="rounded border-gray-600 bg-gray-700"
                  />
                </th>
                {(
                  [
                    { col: "title", label: "Title" },
                    { col: "status", label: "Status" },
                    { col: "quality", label: "Quality" },
                    { col: "size", label: "Size" },
                    { col: "addedAt", label: "Added" },
                  ] as const
                ).map(({ col, label }) => (
                  <th key={col} className="px-3 py-2">
                    <button
                      onClick={() => toggleSort(col)}
                      className="text-xs font-medium text-gray-400 hover:text-gray-200 flex items-center gap-1"
                    >
                      {label}
                      <span className="text-gray-600">{sortIcon(col)}</span>
                    </button>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {items.map((item) => (
                <MediaItemRow
                  key={item.id}
                  item={item}
                  selected={selectedIds.includes(item.id)}
                  onSelect={toggleSelect}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
