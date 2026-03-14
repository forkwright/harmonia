interface Props {
  isPlaying: boolean;
  onPlayPause: () => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
  onPrevChapter: () => void;
  onNextChapter: () => void;
}

export default function AudiobookTransport({
  isPlaying,
  onPlayPause,
  onSkipBack,
  onSkipForward,
  onPrevChapter,
  onNextChapter,
}: Props) {
  return (
    <div className="flex items-center justify-center gap-4">
      <button
        type="button"
        onClick={onPrevChapter}
        className="text-gray-400 hover:text-white transition-colors p-2"
        title="Previous chapter"
      >
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
        </svg>
      </button>

      <button
        type="button"
        onClick={onSkipBack}
        className="text-gray-400 hover:text-white transition-colors p-2 relative"
        title="Skip back 30s"
      >
        <svg className="w-7 h-7" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 5V1L7 6l5 5V7c3.31 0 6 2.69 6 6s-2.69 6-6 6-6-2.69-6-6H4c0 4.42 3.58 8 8 8s8-3.58 8-8-3.58-8-8-8z" />
        </svg>
        <span className="absolute bottom-0 left-1/2 -translate-x-1/2 text-[9px] font-bold">
          30
        </span>
      </button>

      <button
        type="button"
        onClick={onPlayPause}
        className="w-14 h-14 rounded-full bg-white text-gray-900 flex items-center justify-center hover:bg-gray-200 transition-colors"
        title={isPlaying ? "Pause" : "Play"}
      >
        {isPlaying ? (
          <svg className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
            <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
          </svg>
        ) : (
          <svg className="w-6 h-6 ml-1" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
      </button>

      <button
        type="button"
        onClick={onSkipForward}
        className="text-gray-400 hover:text-white transition-colors p-2 relative"
        title="Skip forward 30s"
      >
        <svg className="w-7 h-7" viewBox="0 0 24 24" fill="currentColor">
          <path d="M18 13c0 3.31-2.69 6-6 6s-6-2.69-6-6 2.69-6 6-6v4l5-5-5-5v4c-4.42 0-8 3.58-8 8s3.58 8 8 8 8-3.58 8-8h-2z" />
        </svg>
        <span className="absolute bottom-0 left-1/2 -translate-x-1/2 text-[9px] font-bold">
          30
        </span>
      </button>

      <button
        type="button"
        onClick={onNextChapter}
        className="text-gray-400 hover:text-white transition-colors p-2"
        title="Next chapter"
      >
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
        </svg>
      </button>
    </div>
  );
}
