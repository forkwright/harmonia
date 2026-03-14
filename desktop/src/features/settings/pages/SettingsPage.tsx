import AboutSection from "../components/AboutSection";
import AudioSettings from "../components/AudioSettings";
import GeneralSettings from "../components/GeneralSettings";
import NotificationSettings from "../components/NotificationSettings";
import SystemSettings from "../components/SystemSettings";
import { useSettings } from "../hooks/useSettings";

export default function SettingsPage() {
  const { config, appInfo, loading, saveStatus, updateConfig, updateNotifications, setAutoStart } =
    useSettings();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <span className="text-gray-500 text-sm">Loading settings…</span>
      </div>
    );
  }

  const statusText: Record<typeof saveStatus, string> = {
    idle: "",
    saving: "Saving…",
    saved: "Saved",
    error: "Save failed",
  };

  const statusColor: Record<typeof saveStatus, string> = {
    idle: "text-transparent",
    saving: "text-gray-400",
    saved: "text-green-400",
    error: "text-red-400",
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-10">
        <div className="flex items-center justify-between">
          <h1 className="text-xl font-semibold text-gray-100">Settings</h1>
          <span className={`text-xs transition-colors ${statusColor[saveStatus]}`}>
            {statusText[saveStatus]}
          </span>
        </div>

        <GeneralSettings config={config} onUpdate={updateConfig} />

        <SystemSettings
          config={config}
          onUpdate={updateConfig}
          onAutoStartChange={setAutoStart}
        />

        <NotificationSettings
          config={config.notifications}
          onUpdate={updateNotifications}
        />

        <AudioSettings />

        <AboutSection appInfo={appInfo} />
      </div>
    </div>
  );
}
