import { useCallback, useEffect, useRef, useState } from "react";
import { usePlayback } from "../hooks/usePlayback";

interface Props {
  positionMs: number;
  durationMs: number;
}

function formatTime(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export default function ProgressBar({ positionMs, durationMs }: Props) {
  const { seek } = usePlayback();
  const barRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  const [dragPosition, setDragPosition] = useState(0);
  const [showTotal, setShowTotal] = useState(false);

  const fraction =
    durationMs > 0 ? Math.min((dragging ? dragPosition : positionMs) / durationMs, 1) : 0;

  function seekToFraction(clientX: number) {
    if (!barRef.current || durationMs === 0) return;
    const rect = barRef.current.getBoundingClientRect();
    const f = Math.max(0, Math.min((clientX - rect.left) / rect.width, 1));
    const targetMs = Math.round(f * durationMs);
    seek(targetMs);
  }

  function handleClick(e: React.MouseEvent<HTMLDivElement>) {
    seekToFraction(e.clientX);
  }

  function handleMouseDown(e: React.MouseEvent<HTMLDivElement>) {
    setDragging(true);
    const f = Math.max(
      0,
      Math.min(
        (e.clientX - barRef.current!.getBoundingClientRect().left) /
          barRef.current!.getBoundingClientRect().width,
        1
      )
    );
    setDragPosition(f * durationMs);
    e.preventDefault();
  }

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!dragging || !barRef.current || durationMs === 0) return;
      const rect = barRef.current.getBoundingClientRect();
      const f = Math.max(0, Math.min((e.clientX - rect.left) / rect.width, 1));
      setDragPosition(f * durationMs);
    },
    [dragging, durationMs]
  );

  const handleMouseUp = useCallback(
    (e: MouseEvent) => {
      if (!dragging) return;
      setDragging(false);
      seekToFraction(e.clientX);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [dragging, durationMs]
  );

  useEffect(() => {
    if (dragging) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    }
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragging, handleMouseMove, handleMouseUp]);

  const displayPosition = dragging ? dragPosition : positionMs;
  const remaining = durationMs - displayPosition;

  return (
    <div className="flex items-center gap-2 w-full max-w-sm">
      <span className="text-xs text-gray-500 tabular-nums w-10 text-right">
        {formatTime(displayPosition)}
      </span>

      {/* 16px hit-target bar */}
      <div
        ref={barRef}
        className="flex-1 h-4 flex items-center cursor-pointer group"
        onClick={handleClick}
        onMouseDown={handleMouseDown}
        role="slider"
        aria-label="Playback position"
        aria-valuenow={Math.round(displayPosition)}
        aria-valuemin={0}
        aria-valuemax={durationMs}
      >
        <div className="relative w-full h-1 bg-gray-700 rounded group-hover:h-1.5 transition-all">
          <div
            className="absolute left-0 top-0 h-full bg-white rounded"
            style={{ width: `${fraction * 100}%` }}
          />
          {/* Drag handle */}
          <div
            className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-3 h-3 bg-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
            style={{ left: `${fraction * 100}%` }}
          />
        </div>
      </div>

      <button
        className="text-xs text-gray-500 tabular-nums w-12 text-left hover:text-gray-300 transition-colors"
        onClick={() => setShowTotal((v) => !v)}
        aria-label="Toggle time display"
      >
        {showTotal ? formatTime(durationMs) : `-${formatTime(remaining)}`}
      </button>
    </div>
  );
}
