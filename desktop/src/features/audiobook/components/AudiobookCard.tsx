import clsx from "clsx";
import type { Audiobook } from "../../../types/media";

function formatDuration(ms: number): string {
  const totalMins = Math.floor(ms / 60_000);
  const hours = Math.floor(totalMins / 60);
  const mins = totalMins % 60;
  return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
}

function progressColorClass(progress: Audiobook["progress"]): string {
  if (!progress) return "bg-gray-600";
  if (progress.completedAt) return "bg-green-500";
  if (progress.percentComplete > 0) return "bg-blue-500";
  return "bg-gray-600";
}

interface Props {
  book: Audiobook;
  onClick?: () => void;
}

export default function AudiobookCard({ book, onClick }: Props) {
  const initial = book.title.charAt(0).toUpperCase();
  const pct = book.progress?.percentComplete ?? 0;

  return (
    <div
      className={clsx(
        "bg-gray-800 rounded-lg overflow-hidden cursor-pointer",
        "hover:bg-gray-700 transition-colors group"
      )}
      onClick={onClick}
    >
      <div className="relative aspect-square bg-gray-700 flex items-center justify-center">
        {book.coverUrl ? (
          <img
            src={book.coverUrl}
            alt={book.title}
            className="w-full h-full object-cover"
          />
        ) : (
          <span className="text-4xl font-bold text-gray-500 group-hover:text-gray-400 select-none">
            {initial}
          </span>
        )}
        {pct > 0 && (
          <div className="absolute bottom-0 left-0 right-0 h-1">
            <div
              className={clsx("h-full", progressColorClass(book.progress))}
              style={{ width: `${Math.min(100, pct)}%` }}
            />
          </div>
        )}
      </div>
      <div className="p-3">
        <p className="text-sm font-medium text-white truncate" title={book.title}>
          {book.title}
        </p>
        <p className="text-xs text-gray-400 truncate mt-0.5">{book.author}</p>
        {book.seriesName && (
          <p className="text-xs text-gray-500 truncate mt-0.5">
            {book.seriesName}
            {book.seriesPosition != null ? ` #${book.seriesPosition}` : ""}
          </p>
        )}
        <p className="text-xs text-gray-500 mt-1">{formatDuration(book.durationMs)}</p>
      </div>
    </div>
  );
}
