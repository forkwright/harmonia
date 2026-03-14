import { useState } from "react";
import { useRequests } from "../hooks/useRequests";
import { useManagementStore } from "../store";
import RequestRow from "../components/RequestRow";
import type { MediaRequest, MediaType } from "../../../types/management";

type TabStatus = "pending" | "approved" | "denied";

const TABS: { status: TabStatus; label: string }[] = [
  { status: "pending", label: "Pending" },
  { status: "approved", label: "Approved" },
  { status: "denied", label: "Denied" },
];

const MEDIA_TYPES: MediaType[] = [
  "music", "audiobook", "ebook", "podcast", "manga", "news", "movie", "tv",
];

export default function RequestsPage() {
  const isAdmin = useManagementStore((s) => s.isAdmin);
  const [tab, setTab] = useState<TabStatus>("pending");
  const [newTitle, setNewTitle] = useState("");
  const [newType, setNewType] = useState<MediaType>("movie");
  const [newExternalId, setNewExternalId] = useState("");

  const { requests, create, approve, deny, cancel } = useRequests({ status: tab });

  function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!newTitle.trim()) return;
    create.mutate(
      {
        mediaType: newType,
        title: newTitle.trim(),
        externalId: newExternalId.trim() || undefined,
      },
      {
        onSuccess: () => {
          setNewTitle("");
          setNewExternalId("");
        },
      },
    );
  }

  const items: MediaRequest[] = requests.data?.data ?? [];

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Request Queue</h1>

      <form onSubmit={handleCreate} className="p-4 rounded border border-gray-700 bg-gray-800/50 space-y-3">
        <h2 className="text-sm font-medium text-gray-300">New Request</h2>
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
            value={newExternalId}
            onChange={(e) => setNewExternalId(e.target.value)}
            placeholder="External ID (IMDB, TMDB…)"
            className="w-52 px-3 py-1.5 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <button
            type="submit"
            disabled={create.isPending || !newTitle.trim()}
            className="px-4 py-1.5 text-sm rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
          >
            Submit
          </button>
        </div>
      </form>

      <div className="flex gap-1 border-b border-gray-800">
        {TABS.map(({ status, label }) => (
          <button
            key={status}
            onClick={() => setTab(status)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              tab === status
                ? "border-blue-500 text-white"
                : "border-transparent text-gray-400 hover:text-white"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {requests.isLoading && <p className="text-sm text-gray-500">Loading…</p>}
      {requests.isError && <p className="text-sm text-red-400">Failed to load requests.</p>}
      {!requests.isLoading && items.length === 0 && (
        <p className="text-sm text-gray-500">No {tab} requests.</p>
      )}
      <div className="space-y-3">
        {items.map((req) => (
          <RequestRow
            key={req.id}
            request={req}
            isAdmin={isAdmin}
            onApprove={(id) => approve.mutate(id)}
            onDeny={(id, reason) => deny.mutate({ requestId: id, reason })}
            onCancel={(id) => cancel.mutate(id)}
          />
        ))}
      </div>
    </div>
  );
}
