import { useNavigate } from "react-router-dom";
import type { TrackInfo as TrackInfoType } from "../../../types/playback";

interface Props {
  track: TrackInfoType | null;
}

export default function TrackInfo({ track }: Props) {
  const navigate = useNavigate();

  if (!track) {
    return (
      <div className="flex items-center gap-3 w-56 min-w-0">
        <div className="w-10 h-10 bg-gray-800 rounded flex-shrink-0" />
        <div className="min-w-0">
          <p className="text-sm text-gray-500">Nothing playing</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3 w-56 min-w-0">
      <div className="w-10 h-10 bg-gray-700 rounded flex-shrink-0 flex items-center justify-center">
        <span className="text-xs text-gray-400">♪</span>
      </div>
      <div className="min-w-0">
        <button
          onClick={() => track.album && navigate(`/library/albums`)}
          className="block text-sm text-white truncate hover:underline text-left w-full"
        >
          {track.title}
        </button>
        {track.artist && (
          <button
            onClick={() => navigate(`/library/tracks`)}
            className="block text-xs text-gray-400 truncate hover:underline text-left w-full"
          >
            {track.artist}
          </button>
        )}
      </div>
    </div>
  );
}
