import { Link } from "react-router-dom";
import type { SignalPathInfo } from "../../../types/playback";

interface Props {
  info: SignalPathInfo;
}

function tierColor(tier: string): string {
  switch (tier) {
    case "Bit-Perfect":
    case "Lossless":
      return "text-green-400";
    case "High Quality":
      return "text-yellow-400";
    default:
      return "text-red-400";
  }
}

export default function QualityIndicator({ info }: Props) {
  if (!info.source_codec) return null;

  return (
    <Link
      to="/signal-path"
      className={`flex items-center gap-1 text-xs font-mono ${tierColor(info.quality_tier)} hover:opacity-80 transition-opacity`}
      title="Signal path"
    >
      {info.is_bit_perfect && (
        <span className="bg-green-900/50 text-green-300 px-1 py-0.5 rounded text-[10px] font-semibold">
          BIT-PERFECT
        </span>
      )}
      <span>{info.source_codec}</span>
      <span className="text-gray-500">
        {Math.round(info.source_sample_rate / 1000)}kHz
      </span>
    </Link>
  );
}
