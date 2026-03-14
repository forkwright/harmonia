import type { Indexer, IndexerTestResult } from "../../../types/management";

const STATUS_COLORS: Record<Indexer["status"], string> = {
  active: "bg-green-900 text-green-300",
  degraded: "bg-yellow-900 text-yellow-300",
  failed: "bg-red-900 text-red-300",
};

interface Props {
  indexer: Indexer;
  testResult: IndexerTestResult | null;
  onTest: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
  onMoveUp: (id: string) => void;
  onMoveDown: (id: string) => void;
  testing: boolean;
  isFirst: boolean;
  isLast: boolean;
}

export default function IndexerRow({
  indexer,
  testResult,
  onTest,
  onToggle,
  onDelete,
  onMoveUp,
  onMoveDown,
  testing,
  isFirst,
  isLast,
}: Props) {
  return (
    <div className="p-3 rounded bg-gray-800/50 border border-gray-700 space-y-2">
      <div className="flex items-center gap-3">
        <div className="flex flex-col gap-0.5">
          <button
            onClick={() => onMoveUp(indexer.id)}
            disabled={isFirst}
            className="text-gray-500 hover:text-gray-300 disabled:opacity-30 text-xs leading-none"
            aria-label="Move up"
          >
            ▲
          </button>
          <button
            onClick={() => onMoveDown(indexer.id)}
            disabled={isLast}
            className="text-gray-500 hover:text-gray-300 disabled:opacity-30 text-xs leading-none"
            aria-label="Move down"
          >
            ▼
          </button>
        </div>
        <span className="text-xs text-gray-500 w-6 text-center">{indexer.priority}</span>
        <div className="flex-1 min-w-0">
          <p className="text-sm text-gray-100">{indexer.name}</p>
          <p className="text-xs text-gray-400 truncate">{indexer.url}</p>
        </div>
        <span className="text-xs text-gray-500 uppercase">{indexer.protocol}</span>
        <span className={`px-2 py-0.5 text-xs font-medium rounded ${STATUS_COLORS[indexer.status]}`}>
          {indexer.status}
        </span>
        <label className="flex items-center gap-1 cursor-pointer">
          <input
            type="checkbox"
            checked={indexer.enabled}
            onChange={(e) => onToggle(indexer.id, e.target.checked)}
            className="rounded border-gray-600 bg-gray-700"
          />
          <span className="text-xs text-gray-400">Enabled</span>
        </label>
        <button
          onClick={() => onTest(indexer.id)}
          disabled={testing}
          className="px-2 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 transition-colors"
        >
          Test
        </button>
        <button
          onClick={() => onDelete(indexer.id)}
          className="px-2 py-1 text-xs rounded bg-red-900 hover:bg-red-800 text-red-300 transition-colors"
        >
          Delete
        </button>
      </div>
      {testResult && (
        <p className={`text-xs ${testResult.success ? "text-green-400" : "text-red-400"}`}>
          {testResult.success
            ? `OK — ${testResult.responseTimeMs}ms, ${testResult.categories} categories`
            : `Failed: ${testResult.error}`}
        </p>
      )}
    </div>
  );
}
