import type { SearchResult, MediaType } from "../../../types/management";

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatAge(iso: string): string {
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
  if (days === 0) return "today";
  if (days === 1) return "1d";
  if (days < 30) return `${days}d`;
  if (days < 365) return `${Math.floor(days / 30)}mo`;
  return `${Math.floor(days / 365)}y`;
}

interface Props {
  result: SearchResult;
  onGrab: (result: SearchResult, mediaType: MediaType) => void;
  grabbing: boolean;
  selectedMediaType: MediaType;
}

export default function SearchResultRow({ result, onGrab, grabbing, selectedMediaType }: Props) {
  return (
    <tr className="border-b border-gray-800 hover:bg-gray-800/50">
      <td className="px-3 py-2 text-sm text-gray-100">{result.title}</td>
      <td className="px-3 py-2 text-xs text-gray-400">{result.indexerName}</td>
      <td className="px-3 py-2 text-sm text-gray-300">{formatBytes(result.size)}</td>
      <td className="px-3 py-2 text-sm text-green-400">{result.seeders}</td>
      <td className="px-3 py-2 text-sm text-gray-400">{result.leechers}</td>
      <td className="px-3 py-2 text-xs text-gray-300">{result.quality ?? "—"}</td>
      <td className="px-3 py-2 text-xs text-gray-400">{formatAge(result.publicationDate)}</td>
      <td className="px-3 py-2 text-xs text-gray-500 uppercase">{result.protocol}</td>
      <td className="px-3 py-2">
        <button
          onClick={() => onGrab(result, selectedMediaType)}
          disabled={grabbing}
          className="px-3 py-1 text-xs rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
        >
          Grab
        </button>
      </td>
    </tr>
  );
}
