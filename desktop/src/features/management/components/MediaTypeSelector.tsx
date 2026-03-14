import { useManagementStore } from "../store";
import type { MediaType } from "../../../types/management";

const MEDIA_TYPES: { type: MediaType; label: string }[] = [
  { type: "music", label: "Music" },
  { type: "audiobook", label: "Audiobooks" },
  { type: "ebook", label: "Ebooks" },
  { type: "podcast", label: "Podcasts" },
  { type: "manga", label: "Manga" },
  { type: "news", label: "News" },
  { type: "movie", label: "Movies" },
  { type: "tv", label: "TV" },
];

interface Props {
  counts?: Partial<Record<MediaType, number>>;
}

export default function MediaTypeSelector({ counts }: Props) {
  const selected = useManagementStore((s) => s.selectedMediaType);
  const setSelectedMediaType = useManagementStore((s) => s.setSelectedMediaType);

  return (
    <div className="flex gap-1 border-b border-gray-800 overflow-x-auto">
      {MEDIA_TYPES.map(({ type, label }) => (
        <button
          key={type}
          onClick={() => setSelectedMediaType(type)}
          className={`px-4 py-2 text-sm font-medium whitespace-nowrap border-b-2 transition-colors ${
            selected === type
              ? "border-blue-500 text-white"
              : "border-transparent text-gray-400 hover:text-white hover:border-gray-600"
          }`}
        >
          {label}
          {counts?.[type] !== undefined && (
            <span className="ml-2 px-1.5 py-0.5 text-xs rounded-full bg-gray-700 text-gray-300">
              {counts[type]}
            </span>
          )}
        </button>
      ))}
    </div>
  );
}
