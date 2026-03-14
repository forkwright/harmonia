import type { DownloadStatus } from "../../../types/management";

const STATUS_COLORS: Record<DownloadStatus["status"], string> = {
  queued: "text-gray-400",
  downloading: "text-blue-400",
  extracting: "text-purple-400",
  importing: "text-yellow-400",
  completed: "text-green-400",
  failed: "text-red-400",
};

function formatSpeed(bytesPerSec: number | null): string {
  if (bytesPerSec === null) return "—";
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

interface Props {
  download: DownloadStatus;
  onCancel?: (id: string) => void;
  onRetry?: (id: string) => void;
  cancelling?: boolean;
  retrying?: boolean;
}

export default function DownloadRow({ download, onCancel, onRetry, cancelling, retrying }: Props) {
  return (
    <div className="flex flex-col gap-1 p-3 rounded bg-gray-800/50 border border-gray-700">
      <div className="flex items-center justify-between gap-4">
        <span className="text-sm text-gray-100 truncate flex-1">{download.title}</span>
        <span className={`text-xs font-medium ${STATUS_COLORS[download.status]}`}>
          {download.status}
        </span>
        <div className="flex gap-2 flex-shrink-0">
          {download.status === "failed" && onRetry && (
            <button
              onClick={() => onRetry(download.id)}
              disabled={retrying}
              className="px-2 py-1 text-xs rounded bg-yellow-700 hover:bg-yellow-600 disabled:opacity-50 text-white transition-colors"
            >
              Retry
            </button>
          )}
          {(download.status === "queued" || download.status === "downloading") && onCancel && (
            <button
              onClick={() => onCancel(download.id)}
              disabled={cancelling}
              className="px-2 py-1 text-xs rounded bg-red-800 hover:bg-red-700 disabled:opacity-50 text-white transition-colors"
            >
              Cancel
            </button>
          )}
        </div>
      </div>
      {download.status === "downloading" && (
        <div className="flex items-center gap-3">
          <div className="flex-1 bg-gray-700 rounded-full h-1.5">
            <div
              className="bg-blue-500 h-1.5 rounded-full transition-all"
              style={{ width: `${download.progressPercent}%` }}
            />
          </div>
          <span className="text-xs text-gray-400 w-10 text-right">
            {download.progressPercent.toFixed(0)}%
          </span>
          <span className="text-xs text-gray-500">{formatSpeed(download.downloadSpeed)}</span>
          {download.eta && <span className="text-xs text-gray-500">{download.eta}</span>}
        </div>
      )}
    </div>
  );
}
