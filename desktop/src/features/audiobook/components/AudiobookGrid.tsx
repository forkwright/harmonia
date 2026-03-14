import { useCallback } from "react";
import { VirtuosoGrid } from "react-virtuoso";
import AudiobookCard from "./AudiobookCard";
import type { Audiobook } from "../../../types/media";

interface Props {
  books: Audiobook[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
  onSelect: (book: Audiobook) => void;
}

export default function AudiobookGrid({
  books,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
  onSelect,
}: Props) {
  const endReached = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <VirtuosoGrid
      style={{ height: "100%" }}
      totalCount={books.length}
      endReached={endReached}
      itemContent={(index) => (
        <div className="p-2">
          <AudiobookCard book={books[index]} onClick={() => onSelect(books[index])} />
        </div>
      )}
      listClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6"
      components={{
        Footer: () =>
          isFetchingNextPage ? (
            <div className="py-4 text-center text-sm text-gray-500">Loading…</div>
          ) : null,
      }}
    />
  );
}
