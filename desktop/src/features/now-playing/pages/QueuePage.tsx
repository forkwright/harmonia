import { useRef, useState } from "react";
import type { QueueEntry } from "../../../types/playback";
import { usePlayback } from "../hooks/usePlayback";
import { useQueue } from "../hooks/useQueue";

export default function QueuePage() {
  const queue = useQueue();
  const { queueRemove, queueMove, queueClear } = usePlayback();

  const dragIndex = useRef<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);

  function handleDragStart(index: number) {
    dragIndex.current = index;
  }

  function handleDragOver(e: React.DragEvent, index: number) {
    e.preventDefault();
    setDragOver(index);
  }

  function handleDrop(toIndex: number) {
    const from = dragIndex.current;
    if (from !== null && from !== toIndex) {
      queueMove(from, toIndex);
    }
    dragIndex.current = null;
    setDragOver(null);
  }

  function handleDragEnd() {
    dragIndex.current = null;
    setDragOver(null);
  }

  if (queue.entries.length === 0) {
    return (
      <div className="flex flex-col h-full">
        <QueueHeader onClear={() => queueClear()} count={0} sourceLabel="" />
        <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
          Queue is empty
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <QueueHeader
        onClear={() => queueClear()}
        count={queue.entries.length}
        sourceLabel={queue.source_label}
      />
      <div className="flex-1 overflow-y-auto">
        {queue.entries.map((entry, index) => (
          <QueueRow
            key={`${entry.track_id}-${index}`}
            entry={entry}
            index={index}
            isCurrent={index === queue.current_index}
            isDragOver={dragOver === index}
            onRemove={() => queueRemove(index)}
            onDragStart={() => handleDragStart(index)}
            onDragOver={(e) => handleDragOver(e, index)}
            onDrop={() => handleDrop(index)}
            onDragEnd={handleDragEnd}
          />
        ))}
      </div>
    </div>
  );
}

function QueueHeader({
  onClear,
  count,
  sourceLabel,
}: {
  onClear: () => void;
  count: number;
  sourceLabel: string;
}) {
  return (
    <div className="px-6 py-4 border-b border-gray-800 flex items-center justify-between">
      <div>
        <h2 className="text-sm font-semibold text-white">Queue</h2>
        {sourceLabel && (
          <p className="text-xs text-gray-500 mt-0.5">Playing from: {sourceLabel}</p>
        )}
      </div>
      <div className="flex items-center gap-3">
        <span className="text-xs text-gray-500">{count} tracks</span>
        <button
          onClick={onClear}
          className="text-xs text-gray-400 hover:text-white transition-colors"
        >
          Clear
        </button>
      </div>
    </div>
  );
}

interface RowProps {
  entry: QueueEntry;
  index: number;
  isCurrent: boolean;
  isDragOver: boolean;
  onRemove: () => void;
  onDragStart: () => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: () => void;
  onDragEnd: () => void;
}

function QueueRow({
  entry,
  isCurrent,
  isDragOver,
  onRemove,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
}: RowProps) {
  function formatDuration(ms: number | null): string {
    if (ms == null) return "—";
    const totalSecs = Math.floor(ms / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onDragEnd={onDragEnd}
      className={`flex items-center gap-3 px-6 py-3 border-b border-gray-800/50 cursor-grab active:cursor-grabbing transition-colors group ${
        isCurrent ? "bg-gray-800/70" : "hover:bg-gray-800/40"
      } ${isDragOver ? "border-t-2 border-blue-500" : ""}`}
    >
      {isCurrent && (
        <span className="text-blue-400 text-xs" aria-label="Now playing">
          ▶
        </span>
      )}
      {!isCurrent && <span className="w-3" />}

      <div className="flex-1 min-w-0">
        <p className={`text-sm truncate ${isCurrent ? "text-white" : "text-gray-300"}`}>
          {entry.title}
        </p>
        {entry.artist && (
          <p className="text-xs text-gray-500 truncate">{entry.artist}</p>
        )}
      </div>

      <span className="text-xs text-gray-500 tabular-nums">
        {formatDuration(entry.duration_ms)}
      </span>

      <button
        onClick={onRemove}
        className="text-gray-600 hover:text-gray-300 transition-colors opacity-0 group-hover:opacity-100 text-sm"
        aria-label="Remove from queue"
      >
        ✕
      </button>
    </div>
  );
}
