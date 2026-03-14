import clsx from "clsx";

const SPEED_PRESETS = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0] as const;

interface Props {
  speed: number;
  onSpeedChange: (speed: number) => void;
}

export default function SpeedControl({ speed, onSpeedChange }: Props) {
  return (
    <div className="space-y-2">
      <p className="text-xs text-gray-400 text-center">Speed: {speed.toFixed(2)}x</p>
      <div className="flex flex-wrap gap-1 justify-center">
        {SPEED_PRESETS.map((preset) => (
          <button
            key={preset}
            type="button"
            onClick={() => onSpeedChange(preset)}
            className={clsx(
              "px-2 py-1 rounded text-xs font-medium transition-colors",
              Math.abs(speed - preset) < 0.01
                ? "bg-blue-600 text-white"
                : "bg-gray-700 text-gray-300 hover:bg-gray-600 hover:text-white"
            )}
          >
            {preset}x
          </button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => onSpeedChange(Math.max(0.5, Math.round((speed - 0.05) * 100) / 100))}
          className="px-2 py-1 bg-gray-700 text-gray-300 hover:bg-gray-600 rounded text-xs"
          title="Decrease speed"
        >
          −
        </button>
        <div className="flex-1 text-center text-xs text-gray-500">0.5x – 3.0x</div>
        <button
          type="button"
          onClick={() => onSpeedChange(Math.min(3.0, Math.round((speed + 0.05) * 100) / 100))}
          className="px-2 py-1 bg-gray-700 text-gray-300 hover:bg-gray-600 rounded text-xs"
          title="Increase speed"
        >
          +
        </button>
      </div>
    </div>
  );
}
