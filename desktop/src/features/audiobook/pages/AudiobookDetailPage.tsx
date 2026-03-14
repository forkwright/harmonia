import { useParams, useNavigate } from "react-router-dom";
import { useAudiobook, useAudiobookProgress } from "../hooks/useAudiobook";
import { useAudiobookPlayback } from "../hooks/useAudiobookPlayback";
import { useBookmarks } from "../hooks/useBookmarks";
import ChapterList from "../components/ChapterList";
import BookmarkList from "../components/BookmarkList";
import type { Chapter, Bookmark } from "../../../types/media";

function formatDuration(ms: number): string {
  const totalMins = Math.floor(ms / 60_000);
  const hours = Math.floor(totalMins / 60);
  const mins = totalMins % 60;
  return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-full text-gray-500 text-sm">
      {message}
    </div>
  );
}

export default function AudiobookDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { resume, playFromChapter } = useAudiobookPlayback();

  const audiobookQuery = useAudiobook(id ?? "");
  const progressQuery = useAudiobookProgress(id ?? "");
  const { bookmarks, remove } = useBookmarks(id ?? "");

  const book = audiobookQuery.data?.data;
  const progress = progressQuery.data?.data;

  if (!id) return <EmptyState message="No audiobook selected." />;
  if (audiobookQuery.isLoading) return <EmptyState message="Loading…" />;
  if (audiobookQuery.isError || !book) return <EmptyState message="Audiobook not found." />;

  const chapters = book.chapters;
  const currentChapterPosition = progress?.chapterPosition ?? 0;

  const handleResume = async () => {
    if (progress) {
      await resume(book.id, progress.chapterPosition, progress.offsetMs, chapters);
    } else {
      await playFromChapter(book.id, 0, chapters);
    }
    void navigate("/audiobook-player");
  };

  const handleChapterSelect = async (chapter: Chapter) => {
    await playFromChapter(book.id, chapter.position, chapters);
    void navigate("/audiobook-player");
  };

  const handleBookmarkJump = async (bm: Bookmark) => {
    await resume(book.id, bm.chapterPosition, bm.offsetMs, chapters);
    void navigate("/audiobook-player");
  };

  const resumeLabel = progress
    ? `Resume from Ch. ${progress.chapterPosition + 1} (${Math.round(progress.percentComplete)}%)`
    : "Start from beginning";

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left: cover + metadata + resume */}
      <div className="w-72 flex-shrink-0 flex flex-col bg-gray-900 border-r border-gray-800 overflow-y-auto">
        <div className="p-6 space-y-4">
          <div className="aspect-square bg-gray-800 rounded-lg flex items-center justify-center overflow-hidden">
            {book.coverUrl ? (
              <img src={book.coverUrl} alt={book.title} className="w-full h-full object-cover" />
            ) : (
              <span className="text-6xl font-bold text-gray-600">
                {book.title.charAt(0)}
              </span>
            )}
          </div>

          <div>
            <h1 className="text-lg font-semibold text-white leading-tight">{book.title}</h1>
            <p className="text-sm text-gray-400 mt-1">{book.author}</p>
            {book.narrator && (
              <p className="text-xs text-gray-500 mt-0.5">Narrated by {book.narrator}</p>
            )}
            {book.seriesName && (
              <p className="text-xs text-gray-500 mt-0.5">
                {book.seriesName}
                {book.seriesPosition != null ? ` #${book.seriesPosition}` : ""}
              </p>
            )}
          </div>

          <button
            type="button"
            onClick={() => void handleResume()}
            className="w-full py-3 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium transition-colors text-sm"
          >
            {resumeLabel}
          </button>

          <div className="space-y-1 text-xs text-gray-500">
            <div className="flex justify-between">
              <span>Duration</span>
              <span className="text-gray-400">{formatDuration(book.durationMs)}</span>
            </div>
            <div className="flex justify-between">
              <span>Chapters</span>
              <span className="text-gray-400">{book.chapterCount}</span>
            </div>
            {book.publisher && (
              <div className="flex justify-between">
                <span>Publisher</span>
                <span className="text-gray-400 truncate ml-2">{book.publisher}</span>
              </div>
            )}
            {book.asin && (
              <div className="flex justify-between">
                <span>ASIN</span>
                <span className="text-gray-400">{book.asin}</span>
              </div>
            )}
          </div>

          {book.description && (
            <p className="text-xs text-gray-400 leading-relaxed">{book.description}</p>
          )}
        </div>

        {/* Bookmarks */}
        <div className="border-t border-gray-800 p-4">
          <p className="text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">
            Bookmarks
          </p>
          <BookmarkList
            bookmarks={bookmarks}
            onJump={(bm) => void handleBookmarkJump(bm)}
            onDelete={(bmId) => void remove.mutate(bmId)}
          />
        </div>
      </div>

      {/* Right: chapter list */}
      <div className="flex-1 overflow-hidden flex flex-col">
        <div className="px-4 py-3 border-b border-gray-800 flex-shrink-0">
          <p className="text-sm font-medium text-gray-300">Chapters</p>
        </div>
        <div className="flex-1 overflow-y-auto">
          <ChapterList
            chapters={chapters}
            currentChapterPosition={currentChapterPosition}
            onSelect={(ch) => void handleChapterSelect(ch)}
          />
        </div>
      </div>
    </div>
  );
}
