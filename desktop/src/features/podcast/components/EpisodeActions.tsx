/** Download, mark played/unplayed, and share actions for an episode. */

import type { Episode } from "../../../types/media";
import { useMarkEpisodeCompleted, useMarkEpisodeUnplayed } from "../hooks/useEpisodes";
import { useDownloadEpisode, useDeleteDownload } from "../hooks/useDownloadQueue";

interface Props {
  episode: Episode;
}

export default function EpisodeActions({ episode }: Props) {
  const markCompleted = useMarkEpisodeCompleted();
  const markUnplayed = useMarkEpisodeUnplayed();
  const download = useDownloadEpisode();
  const deleteDownload = useDeleteDownload();

  const isCompleted = episode.progress?.completed ?? false;

  const copyLink = () => {
    void navigator.clipboard.writeText(episode.audioUrl);
  };

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {!episode.downloaded ? (
        <button
          onClick={() => download.mutate(episode.id)}
          disabled={download.isPending}
          className="px-3 py-1.5 rounded-lg bg-gray-700 hover:bg-gray-600 text-sm text-gray-200 disabled:opacity-50 transition-colors"
        >
          {download.isPending ? "Queuing…" : "Download"}
        </button>
      ) : (
        <button
          onClick={() => deleteDownload.mutate(episode.id)}
          disabled={deleteDownload.isPending}
          className="px-3 py-1.5 rounded-lg bg-gray-700 hover:bg-gray-600 text-sm text-gray-200 disabled:opacity-50 transition-colors"
        >
          Delete download
        </button>
      )}

      {isCompleted ? (
        <button
          onClick={() => markUnplayed.mutate(episode.id)}
          disabled={markUnplayed.isPending}
          className="px-3 py-1.5 rounded-lg bg-gray-700 hover:bg-gray-600 text-sm text-gray-200 disabled:opacity-50 transition-colors"
        >
          Mark unplayed
        </button>
      ) : (
        <button
          onClick={() => markCompleted.mutate(episode.id)}
          disabled={markCompleted.isPending}
          className="px-3 py-1.5 rounded-lg bg-gray-700 hover:bg-gray-600 text-sm text-gray-200 disabled:opacity-50 transition-colors"
        >
          Mark played
        </button>
      )}

      <button
        onClick={copyLink}
        className="px-3 py-1.5 rounded-lg bg-gray-700 hover:bg-gray-600 text-sm text-gray-200 transition-colors"
      >
        Copy link
      </button>
    </div>
  );
}
