import clsx from "clsx";

const DURATION_OPTIONS = [5, 10, 15, 30, 45, 60, 90] as const;

interface SleepTimerState {
  endOfChapter: boolean;
  totalSecs: number;
  elapsedSecs: number;
  remainingSecs: number;
  fading: boolean;
}

interface Props {
  timerState: SleepTimerState | null;
  onSetTimer: (minutes: number) => void;
  onSetEndOfChapter: () => void;
  onCancel: () => void;
  onExtend: () => void;
}

function formatRemaining(secs: number): string {
  const mins = Math.floor(secs / 60);
  const s = secs % 60;
  if (mins > 0) return `${mins}:${String(s).padStart(2, "0")}`;
  return `${s}s`;
}

export default function SleepTimerControl({
  timerState,
  onSetTimer,
  onSetEndOfChapter,
  onCancel,
  onExtend,
}: Props) {
  return (
    <div className="space-y-3">
      {timerState ? (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-300">
              {timerState.endOfChapter
                ? "Stopping at chapter end"
                : `Stopping in ${formatRemaining(timerState.remainingSecs)}`}
            </span>
            {timerState.fading && (
              <span className="text-xs text-orange-400 animate-pulse">Fading…</span>
            )}
          </div>
          {!timerState.endOfChapter && timerState.remainingSecs <= 60 && (
            <button
              type="button"
              onClick={onExtend}
              className="w-full py-1.5 bg-gray-700 text-gray-300 hover:bg-gray-600 rounded text-sm transition-colors"
            >
              +5 more minutes
            </button>
          )}
          <button
            type="button"
            onClick={onCancel}
            className="w-full py-1.5 bg-gray-800 text-gray-400 hover:bg-gray-700 rounded text-sm transition-colors"
          >
            Cancel timer
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          <div className="flex flex-wrap gap-1">
            {DURATION_OPTIONS.map((mins) => (
              <button
                key={mins}
                type="button"
                onClick={() => onSetTimer(mins)}
                className={clsx(
                  "px-2 py-1 rounded text-xs font-medium transition-colors",
                  "bg-gray-700 text-gray-300 hover:bg-gray-600 hover:text-white"
                )}
              >
                {mins}m
              </button>
            ))}
          </div>
          <button
            type="button"
            onClick={onSetEndOfChapter}
            className="w-full py-1.5 bg-gray-700 text-gray-300 hover:bg-gray-600 rounded text-sm transition-colors"
          >
            End of chapter
          </button>
        </div>
      )}
    </div>
  );
}
