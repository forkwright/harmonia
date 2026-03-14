import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { usePositionSync } from "../hooks/usePositionSync";
import { useAudiobookPlayback } from "../hooks/useAudiobookPlayback";
import { useChapterNavigation } from "../hooks/useChapterNavigation";
import { useSleepTimer } from "../hooks/useSleepTimer";
import { useBookmarks } from "../hooks/useBookmarks";
import AudiobookTransport from "../components/AudiobookTransport";
import SpeedControl from "../components/SpeedControl";
import SleepTimerControl from "../components/SleepTimerControl";
import BookmarkButton from "../components/BookmarkButton";
import ProgressIndicator from "../components/ProgressIndicator";
import { useAudiobookPlayerStore } from "../store";

function formatMs(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

function formatRemaining(secs: number): string {
  const mins = Math.floor(secs / 60);
  const s = secs % 60;
  if (mins > 0) return `${mins}:${String(s).padStart(2, "0")}`;
  return `${s}s`;
}

export default function AudiobookPlayerPage() {
  const navigate = useNavigate();
  const { position } = usePositionSync();
  const { pause, stop, setSpeed } = useAudiobookPlayback();
  const { nextChapter, prevChapter, skipForward, skipBackward } = useChapterNavigation();
  const { timerState, setTimer, setEndOfChapter, cancel, extendFiveMinutes } = useSleepTimer();
  const storedSpeed = useAudiobookPlayerStore((s) => s.speed);
  const [showChapters, setShowChapters] = useState(false);
  const [showSpeedControl, setShowSpeedControl] = useState(false);
  const [showSleepTimer, setShowSleepTimer] = useState(false);

  const audiobookId = position?.audiobookId ?? "";
  const speed = position?.playbackSpeed ?? storedSpeed(audiobookId);

  const { create: createBookmark } = useBookmarks(audiobookId);

  const addBookmark = useCallback(async () => {
    if (!position) return;
    const label = `Ch. ${position.chapterIndex + 1}, ${formatMs(position.chapterOffsetMs)}`;
    await createBookmark.mutateAsync({
      chapterPosition: position.chapterIndex,
      offsetMs: position.chapterOffsetMs,
      label,
    });
  }, [position, createBookmark]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

      switch (e.key) {
        case " ":
          e.preventDefault();
          if (position?.isPlaying) {
            void pause();
          }
          break;
        case "ArrowRight":
          if (e.shiftKey) {
            void nextChapter();
          } else {
            void skipForward(30);
          }
          break;
        case "ArrowLeft":
          if (e.shiftKey) {
            void prevChapter();
          } else {
            void skipBackward(30);
          }
          break;
        case "]":
          if (position) {
            const newSpeed = Math.min(3.0, Math.round((speed + 0.05) * 100) / 100);
            void setSpeed(audiobookId, newSpeed);
          }
          break;
        case "[":
          if (position) {
            const newSpeed = Math.max(0.5, Math.round((speed - 0.05) * 100) / 100);
            void setSpeed(audiobookId, newSpeed);
          }
          break;
        case "b":
        case "B":
          void addBookmark();
          break;
        case "t":
        case "T":
          setShowSleepTimer((v) => !v);
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    position,
    speed,
    audiobookId,
    pause,
    nextChapter,
    prevChapter,
    skipForward,
    skipBackward,
    setSpeed,
    addBookmark,
  ]);

  if (!position) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <p className="text-gray-400">No audiobook is playing.</p>
        <button
          type="button"
          onClick={() => void navigate("/library/audiobooks")}
          className="px-4 py-2 bg-gray-700 text-gray-300 rounded hover:bg-gray-600 text-sm transition-colors"
        >
          Browse audiobooks
        </button>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Main player */}
      <div className="flex-1 flex flex-col items-center justify-center gap-8 p-8 overflow-y-auto">
        {/* Cover art placeholder */}
        <div className="w-48 h-48 bg-gray-800 rounded-xl flex items-center justify-center shadow-2xl">
          <span className="text-6xl font-bold text-gray-600">
            {position.chapterTitle.charAt(0)}
          </span>
        </div>

        {/* Chapter info */}
        <div className="text-center space-y-1 max-w-sm w-full">
          <p className="text-lg font-semibold text-white truncate">
            {position.chapterTitle}
          </p>
          <p className="text-sm text-gray-400">
            Chapter {position.chapterIndex + 1}
          </p>
        </div>

        {/* Progress */}
        <div className="max-w-sm w-full">
          <ProgressIndicator
            chapterIndex={position.chapterIndex}
            totalChapters={0}
            percentComplete={0}
            chapterTitle={position.chapterTitle}
          />
        </div>

        {/* Transport */}
        <AudiobookTransport
          isPlaying={position.isPlaying}
          onPlayPause={() => void (position.isPlaying ? pause() : Promise.resolve())}
          onSkipBack={() => void skipBackward(30)}
          onSkipForward={() => void skipForward(30)}
          onPrevChapter={() => void prevChapter()}
          onNextChapter={() => void nextChapter()}
        />

        {/* Controls row */}
        <div className="flex items-center gap-4">
          <button
            type="button"
            onClick={() => setShowSpeedControl((v) => !v)}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-gray-800 text-gray-300 hover:bg-gray-700 rounded text-sm transition-colors"
          >
            {speed.toFixed(2)}x
          </button>

          <button
            type="button"
            onClick={() => setShowSleepTimer((v) => !v)}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-gray-800 text-gray-300 hover:bg-gray-700 rounded text-sm transition-colors"
          >
            {timerState ? (
              <span className="text-orange-400">
                {formatRemaining(timerState.remainingSecs)}
              </span>
            ) : (
              "Sleep"
            )}
          </button>

          <BookmarkButton onBookmark={() => void addBookmark()} />

          <button
            type="button"
            onClick={() => setShowChapters((v) => !v)}
            className="px-3 py-1.5 bg-gray-800 text-gray-300 hover:bg-gray-700 rounded text-sm transition-colors"
          >
            Chapters
          </button>

          <button
            type="button"
            onClick={() => void stop()}
            className="px-3 py-1.5 bg-gray-800 text-red-400 hover:bg-gray-700 rounded text-sm transition-colors"
          >
            Stop
          </button>
        </div>

        {/* Speed control panel */}
        {showSpeedControl && (
          <div className="max-w-sm w-full bg-gray-800 rounded-lg p-4">
            <SpeedControl
              speed={speed}
              onSpeedChange={(s) => void setSpeed(audiobookId, s)}
            />
          </div>
        )}

        {/* Sleep timer panel */}
        {showSleepTimer && (
          <div className="max-w-sm w-full bg-gray-800 rounded-lg p-4">
            <SleepTimerControl
              timerState={timerState}
              onSetTimer={(m) => void setTimer(m)}
              onSetEndOfChapter={() => void setEndOfChapter()}
              onCancel={() => void cancel()}
              onExtend={() => void extendFiveMinutes()}
            />
          </div>
        )}
      </div>

      {/* Chapter drawer */}
      {showChapters && (
        <div className="w-72 flex-shrink-0 bg-gray-900 border-l border-gray-800 flex flex-col overflow-hidden">
          <div className="flex items-center justify-between px-4 py-3 border-b border-gray-800">
            <p className="text-sm font-medium text-gray-300">Chapters</p>
            <button
              type="button"
              onClick={() => setShowChapters(false)}
              className="text-gray-500 hover:text-white text-sm"
            >
              Close
            </button>
          </div>
          <div className="flex-1 overflow-y-auto">
            <p className="px-4 py-8 text-center text-sm text-gray-500">
              Open the detail page to see all chapters.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
