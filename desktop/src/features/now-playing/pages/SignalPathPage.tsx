import { useState } from "react";
import type { DspStageInfo } from "../../../types/playback";
import { useSignalPath } from "../hooks/useSignalPath";

export default function SignalPathPage() {
  const info = useSignalPath();

  if (!info.source_codec) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500 text-sm">
        No track playing
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-y-auto px-6 py-6">
      <div className="flex items-center gap-3 mb-6">
        <h2 className="text-lg font-semibold text-white">Signal Path</h2>
        {info.is_bit_perfect && (
          <span className="bg-green-900/50 border border-green-600/50 text-green-300 px-2 py-0.5 rounded text-xs font-semibold">
            BIT-PERFECT
          </span>
        )}
        <span className={`text-xs font-mono ${tierColor(info.quality_tier)}`}>
          {info.quality_tier}
        </span>
      </div>

      {/* Horizontal chain: Source → DSP stages → Output */}
      <div className="flex items-start gap-3 flex-wrap">
        <SourceCard info={info} />

        {info.dsp_stages.map((stage) => (
          <DspStageCard key={stage.name} stage={stage} />
        ))}

        <OutputCard info={info} />
      </div>
    </div>
  );
}

function tierColor(tier: string): string {
  switch (tier) {
    case "Bit-Perfect":
      return "text-green-400";
    case "Lossless":
      return "text-green-400";
    case "High Quality":
      return "text-yellow-400";
    case "Standard":
      return "text-orange-400";
    default:
      return "text-red-400";
  }
}

function tierBorder(tier: string): string {
  switch (tier) {
    case "Bit-Perfect":
    case "Lossless":
      return "border-green-700/50";
    case "High Quality":
      return "border-yellow-700/50";
    case "Standard":
      return "border-orange-700/50";
    default:
      return "border-red-700/50";
  }
}

interface SourceCardProps {
  info: ReturnType<typeof useSignalPath>;
}

function SourceCard({ info }: SourceCardProps) {
  return (
    <StageCard label="Source" tier={info.quality_tier}>
      <p className="text-sm font-mono text-white">{info.source_codec}</p>
      <p className="text-xs text-gray-400 mt-1">
        {Math.round(info.source_sample_rate / 1000)} kHz / {info.source_bit_depth}-bit
      </p>
    </StageCard>
  );
}

function OutputCard({ info }: SourceCardProps) {
  const resampled = info.source_sample_rate !== info.output_sample_rate;
  const tier = resampled ? "High Quality" : info.quality_tier;
  return (
    <StageCard label="Output" tier={tier}>
      <p className="text-sm text-white truncate max-w-[120px]">
        {info.output_device || "Default device"}
      </p>
      <p className="text-xs text-gray-400 mt-1">
        {Math.round(info.output_sample_rate / 1000)} kHz
      </p>
      {resampled && (
        <p className="text-xs text-yellow-400 mt-0.5">Resampled</p>
      )}
    </StageCard>
  );
}

function DspStageCard({ stage }: { stage: DspStageInfo }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <button
      onClick={() => setExpanded((v) => !v)}
      className={`flex flex-col items-start border rounded-lg p-3 min-w-[120px] transition-colors text-left ${
        stage.enabled
          ? "border-gray-700 bg-gray-800/50 hover:bg-gray-800"
          : "border-gray-800 bg-gray-900/50 opacity-50"
      }`}
    >
      <p className="text-xs text-gray-500 uppercase tracking-wider">{stage.name}</p>
      <p className="text-xs text-gray-300 mt-1">
        {stage.enabled ? "Active" : "Bypassed"}
      </p>
      {expanded && stage.parameters && (
        <p className="text-xs text-gray-400 mt-1 font-mono">{stage.parameters}</p>
      )}
    </button>
  );
}

function StageCard({
  label,
  tier,
  children,
}: {
  label: string;
  tier: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`flex flex-col border rounded-lg p-3 min-w-[120px] bg-gray-800/50 ${tierBorder(tier)}`}
    >
      <p className="text-xs text-gray-500 uppercase tracking-wider">{label}</p>
      <div className="mt-1">{children}</div>
    </div>
  );
}
