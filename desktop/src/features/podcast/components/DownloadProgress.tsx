/** Per-episode download indicator pulled from the download queue. */

import { useDownloadQueue } from "../hooks/useDownloadQueue";

interface Props {
  episodeId: string;
}

export default function DownloadProgress({ episodeId }: Props) {
  const { data: queue } = useDownloadQueue();
  const entry = queue?.find((d) => d.episodeId === episodeId);

  if (!entry) return null;

  if (entry.status === "completed") {
    return <span className="text-xs text-green-500 font-medium flex-shrink-0">↓</span>;
  }

  if (entry.status === "failed") {
    return <span className="text-xs text-red-400 flex-shrink-0" title="Download failed">✕</span>;
  }

  return (
    <div className="flex-shrink-0 flex flex-col items-end gap-0.5">
      <span className="text-xs text-blue-400">{Math.round(entry.progressPercent)}%</span>
      <div className="w-12 h-0.5 bg-gray-700 rounded">
        <div
          className="h-0.5 bg-blue-500 rounded transition-all"
          style={{ width: `${entry.progressPercent}%` }}
        />
      </div>
    </div>
  );
}
