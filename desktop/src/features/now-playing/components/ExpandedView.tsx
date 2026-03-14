import { useEffect } from "react";
import type { PlaybackState } from "../../../types/playback";
import { useNowPlayingStore } from "../store";
import ProgressBar from "./ProgressBar";
import TransportControls from "./TransportControls";
import VolumeControl from "./VolumeControl";

interface Props {
  state: PlaybackState;
}

export default function ExpandedView({ state }: Props) {
  const setExpanded = useNowPlayingStore((s) => s.setExpanded);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setExpanded(false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setExpanded]);

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-gray-950/95 backdrop-blur-md"
      onClick={(e) => {
        if (e.target === e.currentTarget) setExpanded(false);
      }}
    >
      {/* Blurred album art background */}
      <div className="absolute inset-0 bg-gradient-to-b from-gray-800/30 to-gray-950 pointer-events-none" />

      <div className="relative z-10 flex flex-col items-center gap-6 w-full max-w-md px-6">
        {/* Large album art */}
        <div className="w-72 h-72 bg-gray-800 rounded-lg flex items-center justify-center shadow-2xl">
          <span className="text-6xl text-gray-600">♪</span>
        </div>

        {/* Track info */}
        <div className="text-center">
          <p className="text-xl font-semibold text-white truncate max-w-sm">
            {state.track?.title ?? "Nothing playing"}
          </p>
          {state.track?.artist && (
            <p className="text-sm text-gray-400 truncate max-w-sm mt-1">
              {state.track.artist}
            </p>
          )}
          {state.track?.album && (
            <p className="text-xs text-gray-500 truncate max-w-sm mt-0.5">
              {state.track.album}
            </p>
          )}
        </div>

        {/* Progress bar */}
        <div className="w-full">
          <ProgressBar
            positionMs={state.position_ms}
            durationMs={state.duration_ms}
          />
        </div>

        {/* Transport controls */}
        <TransportControls
          status={state.status}
          shuffle={state.shuffle}
          repeatMode={state.repeat_mode}
        />

        {/* Volume */}
        <VolumeControl />
      </div>

      {/* Close button */}
      <button
        className="absolute top-4 right-4 text-gray-400 hover:text-white text-2xl transition-colors"
        onClick={() => setExpanded(false)}
        aria-label="Close expanded view"
      >
        ✕
      </button>
    </div>
  );
}
