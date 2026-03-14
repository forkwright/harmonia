/** Podcast subscription tile with cover art and unplayed count badge. */

import { useNavigate } from "react-router-dom";
import type { PodcastSubscription } from "../../../types/media";

interface Props {
  podcast: PodcastSubscription;
}

export default function PodcastCard({ podcast }: Props) {
  const navigate = useNavigate();

  return (
    <button
      onClick={() => navigate(`/library/podcasts/${podcast.id}`)}
      className="group relative w-full text-left rounded-lg overflow-hidden bg-gray-800 hover:bg-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      <div className="aspect-square w-full bg-gray-700 overflow-hidden">
        {podcast.imageUrl ? (
          <img
            src={podcast.imageUrl}
            alt={podcast.title}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-gray-500 text-3xl">
            🎙
          </div>
        )}
      </div>
      <div className="p-2">
        <p className="text-xs font-medium text-gray-100 truncate">{podcast.title}</p>
        {podcast.unplayedCount > 0 && (
          <span className="mt-1 inline-block px-1.5 py-0.5 rounded-full bg-blue-600 text-white text-xs font-semibold">
            {podcast.unplayedCount}
          </span>
        )}
      </div>
    </button>
  );
}
