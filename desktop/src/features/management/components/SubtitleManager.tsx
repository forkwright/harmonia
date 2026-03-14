import { useSubtitles } from "../hooks/useSubtitles";
import type { SubtitleSearchResult } from "../../../types/management";

interface Props {
  mediaId: string;
}

export default function SubtitleManager({ mediaId }: Props) {
  const { subtitles, search, download, remove } = useSubtitles(mediaId);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-gray-200">Subtitles</h3>
        <button
          onClick={() => search.refetch()}
          disabled={search.isFetching}
          className="px-3 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 transition-colors"
        >
          {search.isFetching ? "Searching…" : "Search Subtitles"}
        </button>
      </div>

      {subtitles.isLoading && <p className="text-sm text-gray-500">Loading subtitles…</p>}
      {subtitles.isError && (
        <p className="text-sm text-red-400">Failed to load subtitles.</p>
      )}
      {subtitles.data && subtitles.data.length === 0 && (
        <p className="text-sm text-gray-500">No subtitles downloaded.</p>
      )}
      {subtitles.data && subtitles.data.length > 0 && (
        <ul className="space-y-2">
          {subtitles.data.map((sub) => (
            <li key={sub.id} className="flex items-center justify-between gap-3 text-sm">
              <span className="text-gray-300">
                {sub.language} · {sub.format.toUpperCase()}
              </span>
              <span className="text-xs text-gray-500 truncate flex-1">{sub.filePath}</span>
              <button
                onClick={() => remove.mutate(sub.id)}
                className="px-2 py-0.5 text-xs rounded bg-red-900 hover:bg-red-800 text-red-300 transition-colors"
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}

      {search.data && search.data.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-medium text-gray-400 uppercase tracking-wider">
            Search Results
          </h4>
          <ul className="space-y-1.5">
            {search.data.map((result: SubtitleSearchResult) => (
              <li
                key={result.id}
                className="flex items-center justify-between gap-3 p-2 rounded bg-gray-700/50"
              >
                <span className="text-sm text-gray-300">
                  {result.language} · {result.format.toUpperCase()}
                </span>
                <span className="text-xs text-gray-500">
                  ★ {result.rating.toFixed(1)} · {result.downloadCount} dl
                </span>
                <button
                  onClick={() => download.mutate(result.id)}
                  disabled={download.isPending}
                  className="px-2 py-0.5 text-xs rounded bg-blue-700 hover:bg-blue-600 disabled:opacity-50 text-white transition-colors"
                >
                  Download
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
