import type { AppConfig } from "../hooks/useSettings";

interface Props {
  config: AppConfig;
  onUpdate: (partial: Partial<AppConfig>) => Promise<void>;
  onAutoStartChange: (enabled: boolean) => Promise<void>;
}

interface ToggleRowProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}

function ToggleRow({ label, description, checked, onChange }: ToggleRowProps) {
  return (
    <div className="flex items-center justify-between py-3 border-b border-gray-800 last:border-0">
      <div>
        <p className="text-sm text-gray-200">{label}</p>
        {description && <p className="text-xs text-gray-500 mt-0.5">{description}</p>}
      </div>
      <button
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative w-10 h-5 rounded-full transition-colors focus:outline-none ${
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

export default function SystemSettings({ config, onUpdate, onAutoStartChange }: Props) {
  return (
    <section className="space-y-4">
      <h2 className="text-base font-semibold text-gray-100">System</h2>

      <div className="bg-gray-900 rounded-lg px-4 divide-y divide-gray-800">
        <ToggleRow
          label="Minimize to tray on close"
          description="Keep Harmonia running in the system tray when you close the window."
          checked={config.minimize_to_tray}
          onChange={(v) => onUpdate({ minimize_to_tray: v })}
        />
        <ToggleRow
          label="Start minimized"
          description="Hide the window when Harmonia launches."
          checked={config.start_minimized}
          onChange={(v) => onUpdate({ start_minimized: v })}
        />
        <ToggleRow
          label="Auto-start on login"
          description="Launch Harmonia automatically when you log in."
          checked={config.auto_start}
          onChange={onAutoStartChange}
        />
      </div>
    </section>
  );
}
