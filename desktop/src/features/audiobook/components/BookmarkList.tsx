import type { Bookmark } from "../../../types/media";

function formatMs(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

interface Props {
  bookmarks: Bookmark[];
  onJump: (bookmark: Bookmark) => void;
  onDelete: (bookmarkId: string) => void;
}

export default function BookmarkList({ bookmarks, onJump, onDelete }: Props) {
  if (bookmarks.length === 0) {
    return <p className="text-sm text-gray-500 text-center py-4">No bookmarks yet.</p>;
  }

  return (
    <ul className="space-y-1">
      {bookmarks.map((bm) => (
        <li
          key={bm.id}
          className="flex items-center gap-2 px-3 py-2 rounded hover:bg-gray-800 group"
        >
          <button
            type="button"
            onClick={() => onJump(bm)}
            className="flex-1 text-left"
          >
            <p className="text-sm text-gray-200 truncate">{bm.label}</p>
            <p className="text-xs text-gray-500">
              Ch. {bm.chapterPosition + 1} — {formatMs(bm.offsetMs)}
            </p>
          </button>
          <button
            type="button"
            onClick={() => onDelete(bm.id)}
            className="opacity-0 group-hover:opacity-100 text-gray-500 hover:text-red-400 transition-all p-1"
            title="Delete bookmark"
          >
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z" />
            </svg>
          </button>
        </li>
      ))}
    </ul>
  );
}
