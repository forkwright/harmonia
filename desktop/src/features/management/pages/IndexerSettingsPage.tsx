import { useState } from "react";
import { useIndexers } from "../hooks/useIndexers";
import { useManagementStore } from "../store";
import IndexerRow from "../components/IndexerRow";
import type { IndexerTestResult } from "../../../types/management";

export default function IndexerSettingsPage() {
  const isAdmin = useManagementStore((s) => s.isAdmin);
  const { indexers, add, update, remove, test } = useIndexers();

  const [newName, setNewName] = useState("");
  const [newUrl, setNewUrl] = useState("");
  const [newApiKey, setNewApiKey] = useState("");
  const [newProtocol, setNewProtocol] = useState<"torznab" | "newznab">("torznab");
  const [testResults, setTestResults] = useState<Record<string, IndexerTestResult>>({});
  const [testingId, setTestingId] = useState<string | null>(null);

  if (!isAdmin) {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-sm text-gray-500">Admin access required.</p>
      </div>
    );
  }

  function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!newName.trim() || !newUrl.trim()) return;
    const list = indexers.data ?? [];
    add.mutate(
      {
        name: newName.trim(),
        url: newUrl.trim(),
        apiKey: newApiKey.trim(),
        protocol: newProtocol,
        priority: list.length + 1,
      },
      {
        onSuccess: () => {
          setNewName("");
          setNewUrl("");
          setNewApiKey("");
        },
      },
    );
  }

  function handleTest(id: string) {
    setTestingId(id);
    test.mutate(id, {
      onSuccess: (result) => {
        setTestResults((prev) => ({ ...prev, [id]: result }));
        setTestingId(null);
      },
      onError: () => setTestingId(null),
    });
  }

  function handleMoveUp(id: string) {
    const list = indexers.data ?? [];
    const idx = list.findIndex((x) => x.id === id);
    if (idx <= 0) return;
    const prev = list[idx - 1];
    update.mutate({ id, config: { priority: prev.priority } });
    update.mutate({ id: prev.id, config: { priority: list[idx].priority } });
  }

  function handleMoveDown(id: string) {
    const list = indexers.data ?? [];
    const idx = list.findIndex((x) => x.id === id);
    if (idx < 0 || idx >= list.length - 1) return;
    const next = list[idx + 1];
    update.mutate({ id, config: { priority: next.priority } });
    update.mutate({ id: next.id, config: { priority: list[idx].priority } });
  }

  const list = [...(indexers.data ?? [])].sort((a, b) => a.priority - b.priority);

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Indexer Settings</h1>

      <form
        onSubmit={handleAdd}
        className="p-4 rounded border border-gray-700 bg-gray-800/50 space-y-3"
      >
        <h2 className="text-sm font-medium text-gray-300">Add Indexer</h2>
        <div className="flex gap-3 flex-wrap">
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Name"
            className="w-36 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <input
            type="url"
            value={newUrl}
            onChange={(e) => setNewUrl(e.target.value)}
            placeholder="URL"
            className="flex-1 min-w-52 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <input
            type="text"
            value={newApiKey}
            onChange={(e) => setNewApiKey(e.target.value)}
            placeholder="API Key"
            className="w-44 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <select
            value={newProtocol}
            onChange={(e) => setNewProtocol(e.target.value as "torznab" | "newznab")}
            className="text-sm rounded bg-gray-700 border border-gray-600 text-gray-300 px-2 py-1.5 focus:outline-none"
          >
            <option value="torznab">torznab</option>
            <option value="newznab">newznab</option>
          </select>
          <button
            type="submit"
            disabled={add.isPending || !newName.trim() || !newUrl.trim()}
            className="px-4 py-1.5 text-sm rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
          >
            Add
          </button>
        </div>
      </form>

      {indexers.isLoading && <p className="text-sm text-gray-500">Loading…</p>}
      {indexers.isError && <p className="text-sm text-red-400">Failed to load indexers.</p>}
      {!indexers.isLoading && list.length === 0 && (
        <p className="text-sm text-gray-500">No indexers configured.</p>
      )}
      <div className="space-y-3">
        {list.map((indexer, i) => (
          <IndexerRow
            key={indexer.id}
            indexer={indexer}
            testResult={testResults[indexer.id] ?? null}
            onTest={handleTest}
            onToggle={(id, enabled) => update.mutate({ id, config: { enabled } })}
            onDelete={(id) => remove.mutate(id)}
            onMoveUp={handleMoveUp}
            onMoveDown={handleMoveDown}
            testing={testingId === indexer.id}
            isFirst={i === 0}
            isLast={i === list.length - 1}
          />
        ))}
      </div>
    </div>
  );
}
