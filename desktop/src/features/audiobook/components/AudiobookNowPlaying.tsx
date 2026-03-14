import { useNavigate } from "react-router-dom";
import { usePositionSync } from "../hooks/usePositionSync";
import { useAudiobookPlayback } from "../hooks/useAudiobookPlayback";
import { useChapterNavigation } from "../hooks/useChapterNavigation";

export default function AudiobookNowPlaying() {
  const navigate = useNavigate();
  const { position } = usePositionSync();
  const { pause } = useAudiobookPlayback();
  const { skipForward, skipBackward } = useChapterNavigation();

  if (!position) return null;

  return (
    <div className="flex items-center gap-3 w-full">
      <button
        type="button"
        onClick={() => void navigate("/audiobook-player")}
        className="flex-1 min-w-0 text-left"
      >
        <p className="text-sm font-medium text-white truncate">{position.chapterTitle}</p>
        <p className="text-xs text-gray-400">
          {position.playbackSpeed !== 1 ? `${position.playbackSpeed}x · ` : ""}
          {position.isPlaying ? "Playing" : "Paused"}
        </p>
      </button>

      <div className="flex items-center gap-1">
        <button
          type="button"
          onClick={() => void skipBackward(30)}
          className="p-2 text-gray-400 hover:text-white transition-colors"
          title="Skip back 30s"
        >
          <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 5V1L7 6l5 5V7c3.31 0 6 2.69 6 6s-2.69 6-6 6-6-2.69-6-6H4c0 4.42 3.58 8 8 8s8-3.58 8-8-3.58-8-8-8z" />
          </svg>
        </button>

        <button
          type="button"
          onClick={() => void (position.isPlaying ? pause() : Promise.resolve())}
          className="p-2 text-white hover:text-gray-200 transition-colors"
          title={position.isPlaying ? "Pause" : "Play"}
        >
          {position.isPlaying ? (
            <svg className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
            </svg>
          ) : (
            <svg className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z" />
            </svg>
          )}
        </button>

        <button
          type="button"
          onClick={() => void skipForward(30)}
          className="p-2 text-gray-400 hover:text-white transition-colors"
          title="Skip forward 30s"
        >
          <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M18 13c0 3.31-2.69 6-6 6s-6-2.69-6-6 2.69-6 6-6v4l5-5-5-5v4c-4.42 0-8 3.58-8 8s3.58 8 8 8 8-3.58 8-8h-2z" />
          </svg>
        </button>
      </div>
    </div>
  );
}
