import { useState } from "react";
import { useWanted } from "../hooks/useWanted";
import WantedRow from "../components/WantedRow";
import type { MediaType } from "../../../types/management";

const MEDIA_TYPES: MediaType[] = [
  "music", "audiobook", "ebook", "podcast", "manga", "news", "movie", "tv",
];

export default function WantedPage() {
  const { wanted, add, remove, triggerSearch } = useWanted();
  const [newTitle, setNewTitle] = useState("");
  const [newType, setNewType] = useState<MediaType>("movie");
  const [newProfileId, setNewProfileId] = useState("");
  const [newExternalId, setNewExternalId] = useState("");
  const [searchingId, setSearchingId] = useState<string | null>(null);

  function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!newTitle.trim() || !newProfileId.trim()) return;
    add.mutate(
      {
        mediaType: newType,
        title: newTitle.trim(),
        qualityProfileId: newProfileId.trim(),
        externalId: newExternalId.trim() || undefined,
      },
      {
        onSuccess: () => {
          setNewTitle("");
          setNewProfileId("");
          setNewExternalId("");
        },
      },
    );
  }

  function handleSearch(id: string) {
    setSearchingId(id);
    triggerSearch.mutate(id, { onSettled: () => setSearchingId(null) });
  }

  const items = wanted.data?.data ?? [];

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Wanted List</h1>

      <form
        onSubmit={handleAdd}
        className="p-4 rounded border border-gray-700 bg-gray-800/50 space-y-3"
      >
        <h2 className="text-sm font-medium text-gray-300">Add Wanted</h2>
        <div className="flex gap-3 flex-wrap">
          <select
            value={newType}
            onChange={(e) => setNewType(e.target.value as MediaType)}
            className="text-sm rounded bg-gray-700 border border-gray-600 text-gray-300 px-2 py-1.5 focus:outline-none"
          >
            {MEDIA_TYPES.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
          <input
            type="text"
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder="Title"
            className="flex-1 min-w-40 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <input
            type="text"
            value={newProfileId}
            onChange={(e) => setNewProfileId(e.target.value)}
            placeholder="Quality Profile ID"
            className="w-44 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <input
            type="text"
            value={newExternalId}
            onChange={(e) => setNewExternalId(e.target.value)}
            placeholder="External ID (optional)"
            className="w-48 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <button
            type="submit"
            disabled={add.isPending || !newTitle.trim() || !newProfileId.trim()}
            className="px-4 py-1.5 text-sm rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
          >
            Add
          </button>
        </div>
      </form>

      {wanted.isLoading && <p className="text-sm text-gray-500">Loading…</p>}
      {wanted.isError && <p className="text-sm text-red-400">Failed to load wanted list.</p>}
      {!wanted.isLoading && items.length === 0 && (
        <p className="text-sm text-gray-500">No items on the wanted list.</p>
      )}
      {items.length > 0 && (
        <table className="w-full text-left">
          <thead className="border-b border-gray-700">
            <tr>
              <th className="px-3 py-2 text-xs font-medium text-gray-400">Title</th>
              <th className="px-3 py-2 text-xs font-medium text-gray-400">Type</th>
              <th className="px-3 py-2 text-xs font-medium text-gray-400">Last Searched</th>
              <th className="px-3 py-2 text-xs font-medium text-gray-400">Searches</th>
              <th className="px-3 py-2 text-xs font-medium text-gray-400">Actions</th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <WantedRow
                key={item.id}
                item={item}
                onSearch={handleSearch}
                onRemove={(id) => remove.mutate(id)}
                searching={searchingId === item.id && triggerSearch.isPending}
              />
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
