import type { WantedItem } from "../../../types/management";

function formatDate(iso: string | null): string {
  if (!iso) return "Never";
  return new Date(iso).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

interface Props {
  item: WantedItem;
  onSearch: (id: string) => void;
  onRemove: (id: string) => void;
  searching: boolean;
}

export default function WantedRow({ item, onSearch, onRemove, searching }: Props) {
  return (
    <tr className="border-b border-gray-800 hover:bg-gray-800/50">
      <td className="px-3 py-2 text-sm text-gray-100">{item.title}</td>
      <td className="px-3 py-2 text-xs text-gray-400 capitalize">{item.mediaType}</td>
      <td className="px-3 py-2 text-xs text-gray-400">{formatDate(item.lastSearchedAt)}</td>
      <td className="px-3 py-2 text-xs text-gray-400">{item.searchCount}</td>
      <td className="px-3 py-2">
        <div className="flex gap-2">
          <button
            onClick={() => onSearch(item.id)}
            disabled={searching}
            className="px-2 py-1 text-xs rounded bg-blue-700 hover:bg-blue-600 disabled:opacity-50 text-white transition-colors"
          >
            Search
          </button>
          <button
            onClick={() => onRemove(item.id)}
            className="px-2 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
          >
            Remove
          </button>
        </div>
      </td>
    </tr>
  );
}
