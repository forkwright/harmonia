import { useParams, useNavigate, Link } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useMediaItem } from "../hooks/useMediaItem";
import { useLibraryStore } from "../../library/store";
import MediaStatusBadge from "../components/MediaStatusBadge";
import QualityBadge from "../components/QualityBadge";
import SubtitleManager from "../components/SubtitleManager";
import ActivityFeed from "../components/ActivityFeed";
import { api } from "../../../api/client";

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

const SUBTITLE_TYPES: Set<string> = new Set(["movie", "tv"]);

export default function MediaDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();

  const { data: item, isLoading, isError } = useMediaItem(id ?? "");

  const refreshMetadata = useMutation({
    mutationFn: () => api.refreshMetadata(id ?? "", token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["manage-media-item", id] }),
  });

  const deleteItem = useMutation({
    mutationFn: () => api.deleteMediaItem(id ?? "", token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["manage-media-items"] });
      navigate("/manage/media");
    },
  });

  if (isLoading) {
    return <div className="p-6 text-sm text-gray-500">Loading…</div>;
  }
  if (isError || !item) {
    return <div className="p-6 text-sm text-red-400">Failed to load media item.</div>;
  }

  const showSubtitles = SUBTITLE_TYPES.has(item.mediaType);

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-start gap-4">
        <div className="flex-1 min-w-0">
          <h1 className="text-xl font-semibold text-gray-100">{item.title}</h1>
          <p className="text-sm text-gray-400 capitalize">{item.mediaType}</p>
        </div>
        <div className="flex items-center gap-2 flex-shrink-0">
          <MediaStatusBadge status={item.status} />
          <QualityBadge score={item.qualityScore} />
        </div>
      </div>

      <div className="flex gap-3 flex-wrap">
        <Link
          to={`/manage/media/${id}/edit`}
          className="px-3 py-1.5 text-sm rounded bg-blue-600 hover:bg-blue-500 text-white transition-colors"
        >
          Edit Metadata
        </Link>
        <button
          onClick={() => refreshMetadata.mutate()}
          disabled={refreshMetadata.isPending}
          className="px-3 py-1.5 text-sm rounded bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 transition-colors"
        >
          Refresh Metadata
        </button>
        <Link
          to={`/manage/search?mediaId=${id}`}
          className="px-3 py-1.5 text-sm rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
        >
          Find Upgrade
        </Link>
        <button
          onClick={() => {
            if (confirm("Delete this item?")) deleteItem.mutate();
          }}
          disabled={deleteItem.isPending}
          className="px-3 py-1.5 text-sm rounded bg-red-800 hover:bg-red-700 disabled:opacity-50 text-red-300 transition-colors"
        >
          Delete
        </button>
      </div>

      <section className="grid grid-cols-2 gap-6">
        <div className="space-y-4">
          <div>
            <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
              Metadata
            </h2>
            <dl className="space-y-2">
              {Object.entries(item.fullMetadata).map(([key, val]) => (
                <div key={key} className="grid grid-cols-3 gap-2 text-sm">
                  <dt className="text-gray-500 capitalize">{key.replace(/_/g, " ")}</dt>
                  <dd className="col-span-2 text-gray-300">{String(val ?? "—")}</dd>
                </div>
              ))}
            </dl>
          </div>

          {item.externalIds.length > 0 && (
            <div>
              <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
                External IDs
              </h2>
              <ul className="space-y-1">
                {item.externalIds.map((ext) => (
                  <li key={ext.source} className="flex gap-3 text-sm">
                    <span className="text-gray-500 uppercase w-24">{ext.source}</span>
                    <span className="text-gray-300">{ext.externalId}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>

        <div className="space-y-4">
          <div>
            <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
              Files
            </h2>
            {item.files.length === 0 ? (
              <p className="text-sm text-gray-500">No files tracked.</p>
            ) : (
              <ul className="space-y-2">
                {item.files.map((file, i) => (
                  <li key={i} className="p-2 rounded bg-gray-800/50 text-sm space-y-1">
                    <p className="text-gray-300 truncate" title={file.path}>
                      {file.path}
                    </p>
                    <p className="text-xs text-gray-500">
                      {formatBytes(file.size)}
                      {file.codec && ` · ${file.codec}`}
                      {file.quality && ` · ${file.quality}`}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {item.qualityProfile && (
            <div>
              <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-2">
                Quality Profile
              </h2>
              <p className="text-sm text-gray-300">{item.qualityProfile.name}</p>
              <p className="text-xs text-gray-500">
                Cutoff: {item.qualityProfile.cutoffQualityScore} ·{" "}
                {item.qualityProfile.upgradeAllowed ? "Upgrades allowed" : "No upgrades"}
              </p>
            </div>
          )}
        </div>
      </section>

      {showSubtitles && (
        <section>
          <SubtitleManager mediaId={id ?? ""} />
        </section>
      )}

      {item.history.length > 0 && (
        <section>
          <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
            History
          </h2>
          <ActivityFeed events={item.history} />
        </section>
      )}
    </div>
  );
}
