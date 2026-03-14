import { useNavigate } from "react-router-dom";
import clsx from "clsx";
import { useAudiobooks } from "../hooks/useAudiobooks";
import { useLibraryStore } from "../../library/store";
import { useAudiobookLibraryStore } from "../store";
import { useAudiobookPlayerStore } from "../store";
import AudiobookGrid from "../components/AudiobookGrid";
import type { Audiobook } from "../../../types/media";
import type { FilterOption, SortOption } from "../store";

const FILTER_LABELS: Record<FilterOption, string> = {
  all: "All",
  in_progress: "In Progress",
  completed: "Completed",
  not_started: "Not Started",
};

const SORT_LABELS: Record<SortOption, string> = {
  title: "Title",
  author: "Author",
  recently_listened: "Recently Listened",
  progress: "Progress",
  date_added: "Date Added",
};

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function AudiobooksPage() {
  const navigate = useNavigate();
  const token = useLibraryStore((s) => s.token);
  const filter = useAudiobookLibraryStore((s) => s.filter);
  const sort = useAudiobookLibraryStore((s) => s.sort);
  const setFilter = useAudiobookLibraryStore((s) => s.setFilter);
  const setSort = useAudiobookLibraryStore((s) => s.setSort);
  const recentlyListened = useAudiobookPlayerStore((s) => s.recentlyListened);

  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, isError } =
    useAudiobooks();

  const books = data?.pages.flatMap((p) => p.data) ?? [];

  const recentBooks = books
    .filter((b) => recentlyListened.includes(b.id))
    .sort(
      (a, b) => recentlyListened.indexOf(a.id) - recentlyListened.indexOf(b.id)
    )
    .slice(0, 6);

  const handleSelect = (book: Audiobook) => {
    void navigate(`/library/audiobooks/${book.id}`);
  };

  if (!token) return <EmptyState message="Set an API token in Settings to browse your library." />;
  if (isLoading) return <EmptyState message="Loading…" />;
  if (isError) return <EmptyState message="Failed to load audiobooks." />;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Filter and sort bar */}
      <div className="flex items-center gap-4 px-4 py-3 border-b border-gray-800 flex-shrink-0">
        <div className="flex gap-1">
          {(Object.keys(FILTER_LABELS) as FilterOption[]).map((f) => (
            <button
              key={f}
              type="button"
              onClick={() => setFilter(f)}
              className={clsx(
                "px-3 py-1 rounded text-xs font-medium transition-colors",
                filter === f
                  ? "bg-gray-700 text-white"
                  : "text-gray-400 hover:text-white"
              )}
            >
              {FILTER_LABELS[f]}
            </button>
          ))}
        </div>
        <div className="ml-auto flex items-center gap-2">
          <span className="text-xs text-gray-500">Sort:</span>
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value as SortOption)}
            className="bg-gray-800 text-gray-300 text-xs rounded px-2 py-1 border border-gray-700"
          >
            {(Object.keys(SORT_LABELS) as SortOption[]).map((s) => (
              <option key={s} value={s}>
                {SORT_LABELS[s]}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Continue listening */}
      {recentBooks.length > 0 && (
        <div className="px-4 py-3 border-b border-gray-800 flex-shrink-0">
          <p className="text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">
            Continue Listening
          </p>
          <div className="flex gap-3 overflow-x-auto pb-1">
            {recentBooks.map((book) => (
              <button
                key={book.id}
                type="button"
                onClick={() => handleSelect(book)}
                className="flex-shrink-0 flex items-center gap-2 bg-gray-800 hover:bg-gray-700 rounded-lg p-2 transition-colors max-w-[200px]"
              >
                <div className="w-10 h-10 bg-gray-700 rounded flex items-center justify-center flex-shrink-0">
                  {book.coverUrl ? (
                    <img
                      src={book.coverUrl}
                      alt=""
                      className="w-full h-full object-cover rounded"
                    />
                  ) : (
                    <span className="text-lg font-bold text-gray-500">
                      {book.title.charAt(0)}
                    </span>
                  )}
                </div>
                <div className="min-w-0">
                  <p className="text-xs font-medium text-white truncate">{book.title}</p>
                  {book.progress && (
                    <p className="text-xs text-gray-400">
                      {Math.round(book.progress.percentComplete)}%
                    </p>
                  )}
                </div>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Main grid */}
      <div className="flex-1 overflow-hidden">
        {books.length === 0 ? (
          <EmptyState message="No audiobooks found." />
        ) : (
          <AudiobookGrid
            books={books}
            hasNextPage={hasNextPage ?? false}
            isFetchingNextPage={isFetchingNextPage}
            fetchNextPage={() => void fetchNextPage()}
            onSelect={handleSelect}
          />
        )}
      </div>
    </div>
  );
}
