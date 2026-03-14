import { Link } from "react-router-dom";
import { usePlaybackState } from "../hooks/usePlaybackState";
import { useSignalPath } from "../hooks/useSignalPath";
import { useNowPlayingStore } from "../store";
import ExpandedView from "./ExpandedView";
import ProgressBar from "./ProgressBar";
import QualityIndicator from "./QualityIndicator";
import TrackInfo from "./TrackInfo";
import TransportControls from "./TransportControls";
import VolumeControl from "./VolumeControl";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";

export default function NowPlayingBar() {
  const state = usePlaybackState();
  const signalPath = useSignalPath();
  const expanded = useNowPlayingStore((s) => s.expanded);
  const setExpanded = useNowPlayingStore((s) => s.setExpanded);

  useKeyboardShortcuts(state);

  return (
    <>
      {expanded && <ExpandedView state={state} />}

      <div className="h-20 bg-gray-900 border-t border-gray-800 flex items-center px-4 gap-4 flex-shrink-0">
        {/* Left: track info */}
        <div className="w-56 flex-shrink-0">
          <TrackInfo track={state.track} />
        </div>

        {/* Center: transport + progress */}
        <div className="flex-1 flex flex-col items-center gap-1.5">
          <TransportControls
            status={state.status}
            shuffle={state.shuffle}
            repeatMode={state.repeat_mode}
          />
          <ProgressBar
            positionMs={state.position_ms}
            durationMs={state.duration_ms}
          />
        </div>

        {/* Right: volume, queue, signal path, expand */}
        <div className="w-56 flex-shrink-0 flex items-center justify-end gap-3">
          <VolumeControl />

          <Link
            to="/queue"
            className="text-gray-400 hover:text-white transition-colors text-sm"
            title="Queue"
            aria-label="Open queue"
          >
            ≡
          </Link>

          <QualityIndicator info={signalPath} />

          <button
            onClick={() => setExpanded(true)}
            className="text-gray-400 hover:text-white transition-colors text-sm"
            title="Expand"
            aria-label="Expand now playing"
          >
            ⛶
          </button>
        </div>
      </div>
    </>
  );
}
