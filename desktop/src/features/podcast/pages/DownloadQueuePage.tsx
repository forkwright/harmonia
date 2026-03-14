/** Download queue: status and progress for all podcast episode downloads. */

import { useDownloadQueue, useCancelDownload } from "../hooks/useDownloadQueue";
import { useLibraryStore } from "../../library/store";
import type { EpisodeDownload } from "../../../types/media";

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

function statusLabel(status: EpisodeDownload["status"]): string {
  switch (status) {
    case "queued": return "Queued";
    case "downloading": return "Downloading";
    case "completed": return "Complete";
    case "failed": return "Failed";
  }
}

function statusColor(status: EpisodeDownload["status"]): string {
  switch (status) {
    case "queued": return "text-gray-400";
    case "downloading": return "text-blue-400";
    case "completed": return "text-green-500";
    case "failed": return "text-red-400";
  }
}

export default function DownloadQueuePage() {
  const token = useLibraryStore((s) => s.token);
  const { data: queue, isLoading, isError } = useDownloadQueue();
  const cancel = useCancelDownload();

  if (!token) return <EmptyState message="Set an API token in Settings." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load download queue." />;
  if (!queue || queue.length === 0) return <EmptyState message="No downloads in queue." />;

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 py-3 border-b border-gray-800 flex-shrink-0">
        <h1 className="text-sm font-semibold text-gray-100">Downloads</h1>
      </div>
      <div className="flex-1 overflow-y-auto">
        {queue.map((entry) => (
          <div
            key={entry.episodeId}
            className="flex items-center gap-3 px-4 py-3 border-b border-gray-800"
          >
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-gray-100 truncate">{entry.episodeTitle}</p>
              <p className="text-xs text-gray-500 truncate">{entry.podcastTitle}</p>
            </div>

            <div className="flex-shrink-0 flex items-center gap-3">
              {entry.status === "downloading" && (
                <div className="w-24">
                  <div className="h-1 bg-gray-700 rounded">
                    <div
                      className="h-1 bg-blue-500 rounded transition-all"
                      style={{ width: `${entry.progressPercent}%` }}
                    />
                  </div>
                  <p className="text-xs text-gray-500 mt-0.5 text-right">
                    {Math.round(entry.progressPercent)}%
                  </p>
                </div>
              )}
              <span className={`text-xs font-medium ${statusColor(entry.status)}`}>
                {statusLabel(entry.status)}
              </span>
              {(entry.status === "queued" || entry.status === "downloading") && (
                <button
                  onClick={() => cancel.mutate(entry.episodeId)}
                  disabled={cancel.isPending}
                  className="text-xs text-gray-400 hover:text-gray-200 disabled:opacity-50 transition-colors"
                >
                  Cancel
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
