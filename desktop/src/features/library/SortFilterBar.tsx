import clsx from "clsx";
import { useLibraryStore, type SortOption } from "./store";

const sortOptions: { value: SortOption; label: string }[] = [
  { value: "title", label: "Title" },
  { value: "year", label: "Year" },
  { value: "added", label: "Date Added" },
];

export default function SortFilterBar() {
  const sort = useLibraryStore((s) => s.sort);
  const setSort = useLibraryStore((s) => s.setSort);

  return (
    <div className="flex items-center gap-2 px-6 py-3 border-b border-gray-800">
      <span className="text-xs text-gray-500 uppercase tracking-wide mr-1">Sort</span>
      {sortOptions.map(({ value, label }) => (
        <button
          key={value}
          onClick={() => setSort(value)}
          className={clsx(
            "text-xs px-3 py-1 rounded-full transition-colors",
            sort === value
              ? "bg-blue-600 text-white"
              : "text-gray-400 hover:text-white hover:bg-gray-700"
          )}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
