import type { ScanStatus } from "../../../types/management";

interface Props {
  status: ScanStatus;
  onTrigger: () => void;
  triggering: boolean;
}

export default function ScanProgress({ status, onTrigger, triggering }: Props) {
  return (
    <div className="p-4 rounded border border-gray-700 bg-gray-800/50 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-gray-200">Library Scan</h3>
        <button
          onClick={onTrigger}
          disabled={status.running || triggering}
          className="px-3 py-1 text-xs rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
        >
          {status.running ? "Scanning…" : "Scan Now"}
        </button>
      </div>
      {status.running && (
        <div className="space-y-1">
          <div className="w-full bg-gray-700 rounded-full h-1.5">
            <div className="bg-blue-500 h-1.5 rounded-full animate-pulse w-full" />
          </div>
          <p className="text-xs text-gray-400">
            {status.itemsScanned.toLocaleString()} scanned · {status.itemsAdded} added ·{" "}
            {status.itemsRemoved} removed
          </p>
          {status.estimatedCompletion && (
            <p className="text-xs text-gray-500">
              Est. completion: {new Date(status.estimatedCompletion).toLocaleTimeString()}
            </p>
          )}
        </div>
      )}
      {!status.running && status.startedAt && (
        <p className="text-xs text-gray-500">
          Last scan: {new Date(status.startedAt).toLocaleString()} · {status.itemsScanned} items
        </p>
      )}
    </div>
  );
}
