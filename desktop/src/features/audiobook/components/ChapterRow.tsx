import clsx from "clsx";
import type { Chapter } from "../../../types/media";

function formatMs(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const hours = Math.floor(totalSecs / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hours > 0) {
    return `${hours}:${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

interface Props {
  chapter: Chapter;
  isCurrent: boolean;
  isPlayed: boolean;
  onClick: () => void;
}

export default function ChapterRow({ chapter, isCurrent, isPlayed, onClick }: Props) {
  const durationMs = chapter.endMs - chapter.startMs;

  return (
    <button
      type="button"
      className={clsx(
        "w-full flex items-center gap-3 px-4 py-3 text-left transition-colors",
        isCurrent
          ? "bg-gray-700 text-white"
          : "text-gray-300 hover:bg-gray-800 hover:text-white"
      )}
      onClick={onClick}
    >
      <span
        className={clsx(
          "text-xs font-mono w-6 text-right flex-shrink-0",
          isCurrent ? "text-blue-400" : "text-gray-500"
        )}
      >
        {chapter.position + 1}
      </span>
      <span className={clsx("flex-1 text-sm truncate", isPlayed && !isCurrent && "text-gray-500")}>
        {chapter.title}
      </span>
      <span className="text-xs text-gray-500 flex-shrink-0">{formatMs(durationMs)}</span>
      {isPlayed && !isCurrent && (
        <span className="w-2 h-2 rounded-full bg-blue-500 flex-shrink-0" />
      )}
    </button>
  );
}
