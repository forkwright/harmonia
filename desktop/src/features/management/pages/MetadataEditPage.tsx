import { useParams, useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useMediaItem } from "../hooks/useMediaItem";
import { useLibraryStore } from "../../library/store";
import MetadataForm from "../components/MetadataForm";
import { api } from "../../../api/client";
import type { MetadataUpdate } from "../../../types/management";

export default function MetadataEditPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const { data: item, isLoading, isError } = useMediaItem(id ?? "");

  const save = useMutation({
    mutationFn: (update: MetadataUpdate) => api.updateMetadata(id ?? "", update, token),
    onSuccess: (updated) => {
      queryClient.setQueryData(["manage-media-item", id, token], updated);
      navigate(`/manage/media/${id}`);
    },
  });

  if (isLoading) {
    return <div className="p-6 text-sm text-gray-500">Loading…</div>;
  }
  if (isError || !item) {
    return <div className="p-6 text-sm text-red-400">Failed to load media item.</div>;
  }

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-center gap-3">
        <button
          onClick={() => navigate(-1)}
          className="text-xs text-gray-400 hover:text-gray-200 transition-colors"
        >
          ← Back
        </button>
        <h1 className="text-xl font-semibold text-gray-100">Edit Metadata</h1>
      </div>
      <p className="text-sm text-gray-400">{item.title}</p>
      <MetadataForm item={item} onSave={(update) => save.mutate(update)} saving={save.isPending} />
      {save.isError && (
        <p className="text-sm text-red-400">Failed to save metadata. Please try again.</p>
      )}
    </div>
  );
}
