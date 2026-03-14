import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useManagementStore } from "../store";
import { useIndexerSearch } from "../hooks/useIndexerSearch";
import MediaTypeSelector from "../components/MediaTypeSelector";
import SearchResultRow from "../components/SearchResultRow";
import type { SearchResult, MediaType } from "../../../types/management";

type SortField = "seeders" | "size" | "age" | "quality";

function sortResults(results: SearchResult[], field: SortField, order: "asc" | "desc"): SearchResult[] {
  const sorted = [...results].sort((a, b) => {
    let diff = 0;
    switch (field) {
      case "seeders": diff = a.seeders - b.seeders; break;
      case "size": diff = a.size - b.size; break;
      case "age":
        diff = new Date(a.publicationDate).getTime() - new Date(b.publicationDate).getTime();
        break;
      case "quality":
        diff = (a.quality ?? "").localeCompare(b.quality ?? "");
        break;
    }
    return order === "asc" ? diff : -diff;
  });
  return sorted;
}

export default function SearchPage() {
  const [searchParams] = useSearchParams();
  const selectedMediaType = useManagementStore((s) => s.selectedMediaType);
  const { search, grab } = useIndexerSearch();

  const [query, setQuery] = useState(searchParams.get("q") ?? "");
  const [indexerFilter, setIndexerFilter] = useState("all");
  const [sortField, setSortField] = useState<SortField>("seeders");
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("desc");

  function handleSearch(e: React.FormEvent) {
    e.preventDefault();
    if (!query.trim()) return;
    search.mutate({ query: query.trim(), mediaType: selectedMediaType });
  }

  function handleGrab(result: SearchResult, mediaType: MediaType) {
    grab.mutate({
      searchResultUrl: result.downloadUrl,
      mediaType,
    });
  }

  function toggleSort(field: SortField) {
    if (sortField === field) {
      setSortOrder((o) => (o === "asc" ? "desc" : "asc"));
    } else {
      setSortField(field);
      setSortOrder("desc");
    }
  }

  const results = search.data ?? [];
  const indexerNames = [...new Set(results.map((r) => r.indexerName))];
  const filtered = indexerFilter === "all" ? results : results.filter((r) => r.indexerName === indexerFilter);
  const sorted = sortResults(filtered, sortField, sortOrder);

  function sortIcon(field: SortField): string {
    if (sortField !== field) return "↕";
    return sortOrder === "asc" ? "↑" : "↓";
  }

  return (
    <div className="h-full flex flex-col">
      <div className="border-b border-gray-800">
        <MediaTypeSelector />
      </div>

      <div className="p-4 border-b border-gray-800 space-y-3">
        <form onSubmit={handleSearch} className="flex gap-3">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search indexers…"
            className="flex-1 px-3 py-2 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <button
            type="submit"
            disabled={search.isPending || !query.trim()}
            className="px-4 py-2 text-sm rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
          >
            {search.isPending ? "Searching…" : "Search"}
          </button>
        </form>
        {indexerNames.length > 1 && (
          <div className="flex items-center gap-2">
            <label className="text-xs text-gray-400">Indexer</label>
            <select
              value={indexerFilter}
              onChange={(e) => setIndexerFilter(e.target.value)}
              className="text-xs rounded bg-gray-700 border border-gray-600 text-gray-300 px-2 py-1 focus:outline-none"
            >
              <option value="all">All</option>
              {indexerNames.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-auto">
        {search.isError && (
          <p className="p-4 text-sm text-red-400">Search failed. Check indexer configuration.</p>
        )}
        {!search.isPending && results.length === 0 && search.isSuccess && (
          <p className="p-4 text-sm text-gray-500">No results found.</p>
        )}
        {sorted.length > 0 && (
          <table className="w-full text-left">
            <thead className="sticky top-0 bg-gray-900 border-b border-gray-700">
              <tr>
                <th className="px-3 py-2 text-xs font-medium text-gray-400">Title</th>
                <th className="px-3 py-2 text-xs font-medium text-gray-400">Indexer</th>
                <th className="px-3 py-2">
                  <button onClick={() => toggleSort("size")} className="text-xs font-medium text-gray-400 hover:text-gray-200 flex items-center gap-1">
                    Size <span className="text-gray-600">{sortIcon("size")}</span>
                  </button>
                </th>
                <th className="px-3 py-2">
                  <button onClick={() => toggleSort("seeders")} className="text-xs font-medium text-gray-400 hover:text-gray-200 flex items-center gap-1">
                    Seeds <span className="text-gray-600">{sortIcon("seeders")}</span>
                  </button>
                </th>
                <th className="px-3 py-2 text-xs font-medium text-gray-400">Leech</th>
                <th className="px-3 py-2">
                  <button onClick={() => toggleSort("quality")} className="text-xs font-medium text-gray-400 hover:text-gray-200 flex items-center gap-1">
                    Quality <span className="text-gray-600">{sortIcon("quality")}</span>
                  </button>
                </th>
                <th className="px-3 py-2">
                  <button onClick={() => toggleSort("age")} className="text-xs font-medium text-gray-400 hover:text-gray-200 flex items-center gap-1">
                    Age <span className="text-gray-600">{sortIcon("age")}</span>
                  </button>
                </th>
                <th className="px-3 py-2 text-xs font-medium text-gray-400">Protocol</th>
                <th className="px-3 py-2 text-xs font-medium text-gray-400">Action</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((result, i) => (
                <SearchResultRow
                  key={i}
                  result={result}
                  onGrab={handleGrab}
                  grabbing={grab.isPending}
                  selectedMediaType={selectedMediaType}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
