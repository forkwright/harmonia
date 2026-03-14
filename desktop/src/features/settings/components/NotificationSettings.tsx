import type { NotificationConfig } from "../hooks/useSettings";

interface Props {
  config: NotificationConfig;
  onUpdate: (partial: Partial<NotificationConfig>) => Promise<void>;
}

interface ToggleRowProps {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}

function ToggleRow({ label, description, checked, disabled, onChange }: ToggleRowProps) {
  return (
    <div
      className={`flex items-center justify-between py-3 border-b border-gray-800 last:border-0 ${
        disabled ? "opacity-40" : ""
      }`}
    >
      <div>
        <p className="text-sm text-gray-200">{label}</p>
        {description && <p className="text-xs text-gray-500 mt-0.5">{description}</p>}
      </div>
      <button
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative w-10 h-5 rounded-full transition-colors focus:outline-none disabled:cursor-not-allowed ${
          checked ? "bg-indigo-600" : "bg-gray-700"
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
            checked ? "translate-x-5" : "translate-x-0"
          }`}
        />
      </button>
    </div>
  );
}

export default function NotificationSettings({ config, onUpdate }: Props) {
  return (
    <section className="space-y-4">
      <h2 className="text-base font-semibold text-gray-100">Notifications</h2>

      <div className="bg-gray-900 rounded-lg px-4 divide-y divide-gray-800">
        <ToggleRow
          label="Enable notifications"
          description="Allow Harmonia to send desktop notifications."
          checked={config.enabled}
          onChange={(v) => onUpdate({ enabled: v })}
        />
        <ToggleRow
          label="Track change"
          description="Show a notification when a new track starts playing."
          checked={config.track_change}
          disabled={!config.enabled}
          onChange={(v) => onUpdate({ track_change: v })}
        />
        <ToggleRow
          label="Downloads"
          description="Notify when a download completes."
          checked={config.downloads}
          disabled={!config.enabled}
          onChange={(v) => onUpdate({ downloads: v })}
        />
      </div>
    </section>
  );
}
