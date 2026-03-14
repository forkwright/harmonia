/** Podcast subscriptions grid with subscribe, refresh, sort, and filter controls. */

import { useState, useMemo } from "react";
import { useSubscriptions, useRefreshAllFeeds } from "../hooks/useSubscriptions";
import { useLibraryStore } from "../../library/store";
import PodcastGrid from "../components/PodcastGrid";
import SubscribeDialog from "../components/SubscribeDialog";
import type { PodcastSubscription } from "../../../types/media";

type SortOption = "title" | "recent" | "unplayed";
type FilterOption = "all" | "unplayed" | "recent";

function sortSubscriptions(subs: PodcastSubscription[], sort: SortOption): PodcastSubscription[] {
  return [...subs].sort((a, b) => {
    if (sort === "unplayed") return b.unplayedCount - a.unplayedCount;
    if (sort === "recent") {
      const dateA = a.lastEpisodeDate ?? "";
      const dateB = b.lastEpisodeDate ?? "";
      return dateB.localeCompare(dateA);
    }
    return a.title.localeCompare(b.title);
  });
}

function filterSubscriptions(
  subs: PodcastSubscription[],
  filter: FilterOption,
): PodcastSubscription[] {
  if (filter === "unplayed") return subs.filter((s) => s.unplayedCount > 0);
  if (filter === "recent") {
    const oneWeekAgo = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();
    return subs.filter((s) => (s.lastEpisodeDate ?? "") >= oneWeekAgo);
  }
  return subs;
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function PodcastsPage() {
  const token = useLibraryStore((s) => s.token);
  const { data: subscriptions, isLoading, isError } = useSubscriptions();
  const refreshAll = useRefreshAllFeeds();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [sort, setSort] = useState<SortOption>("title");
  const [filter, setFilter] = useState<FilterOption>("all");

  const displayed = useMemo(() => {
    const filtered = filterSubscriptions(subscriptions ?? [], filter);
    return sortSubscriptions(filtered, sort);
  }, [subscriptions, sort, filter]);

  if (!token) return <EmptyState message="Set an API token in Settings to browse podcasts." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load subscriptions." />;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-3 px-4 py-3 border-b border-gray-800 flex-shrink-0">
        <h1 className="text-sm font-semibold text-gray-100 mr-auto">Podcasts</h1>

        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value as FilterOption)}
          className="text-xs bg-gray-800 text-gray-300 border border-gray-700 rounded px-2 py-1 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="all">All</option>
          <option value="unplayed">Has Unplayed</option>
          <option value="recent">Recently Updated</option>
        </select>

        <select
          value={sort}
          onChange={(e) => setSort(e.target.value as SortOption)}
          className="text-xs bg-gray-800 text-gray-300 border border-gray-700 rounded px-2 py-1 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="title">Title</option>
          <option value="recent">Most Recent Episode</option>
          <option value="unplayed">Unplayed Count</option>
        </select>

        <button
          onClick={() => refreshAll.mutate()}
          disabled={refreshAll.isPending}
          className="px-3 py-1.5 rounded-lg text-xs text-gray-300 bg-gray-800 hover:bg-gray-700 disabled:opacity-50 transition-colors"
        >
          {refreshAll.isPending ? "Refreshing…" : "Refresh all"}
        </button>

        <button
          onClick={() => setDialogOpen(true)}
          className="px-3 py-1.5 rounded-lg text-xs text-white bg-blue-600 hover:bg-blue-500 transition-colors"
        >
          + Subscribe
        </button>
      </div>

      <div className="flex-1 overflow-hidden">
        {displayed.length === 0 ? (
          <EmptyState message="No subscriptions found. Subscribe to a podcast to get started." />
        ) : (
          <PodcastGrid subscriptions={displayed} onEndReached={() => {}} />
        )}
      </div>

      <SubscribeDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </div>
  );
}
