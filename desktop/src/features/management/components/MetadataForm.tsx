import { useState } from "react";
import type { MediaItemDetail, MetadataUpdate } from "../../../types/management";

interface Props {
  item: MediaItemDetail;
  onSave: (update: MetadataUpdate) => void;
  saving: boolean;
}

export default function MetadataForm({ item, onSave, saving }: Props) {
  const [title, setTitle] = useState(item.title);
  const [extra, setExtra] = useState<Record<string, unknown>>(item.fullMetadata);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    onSave({ title, ...extra });
  }

  function handleFieldChange(key: string, value: string) {
    setExtra((prev) => ({ ...prev, [key]: value }));
  }

  const extraFields = Object.entries(item.fullMetadata).filter(
    ([key]) => key !== "title",
  );

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label className="block text-xs text-gray-400 mb-1">Title</label>
        <input
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          className="w-full px-3 py-2 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
        />
      </div>
      {extraFields.map(([key, val]) => (
        <div key={key}>
          <label className="block text-xs text-gray-400 mb-1 capitalize">
            {key.replace(/_/g, " ")}
          </label>
          <input
            type="text"
            value={String(val ?? "")}
            onChange={(e) => handleFieldChange(key, e.target.value)}
            className="w-full px-3 py-2 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
          />
        </div>
      ))}
      <div className="flex gap-3 pt-2">
        <button
          type="submit"
          disabled={saving}
          className="px-4 py-2 text-sm rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
        >
          {saving ? "Saving…" : "Save Changes"}
        </button>
      </div>
    </form>
  );
}
