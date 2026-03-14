import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface NotificationConfig {
  enabled: boolean;
  track_change: boolean;
  downloads: boolean;
}

export interface WindowConfig {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
}

export interface AppConfig {
  server_url: string;
  minimize_to_tray: boolean;
  auto_start: boolean;
  start_minimized: boolean;
  notifications: NotificationConfig;
  window: WindowConfig;
}

export interface AppInfo {
  version: string;
  tauri_version: string;
  os: string;
  arch: string;
}

const defaultConfig: AppConfig = {
  server_url: "http://localhost:7700",
  minimize_to_tray: true,
  auto_start: false,
  start_minimized: false,
  notifications: {
    enabled: true,
    track_change: true,
    downloads: true,
  },
  window: {
    x: 100,
    y: 100,
    width: 1280,
    height: 800,
    maximized: false,
  },
};

export function useSettings() {
  const [config, setConfig] = useState<AppConfig>(defaultConfig);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");

  useEffect(() => {
    Promise.all([
      invoke<AppConfig>("get_app_config"),
      invoke<AppInfo>("get_app_info"),
    ])
      .then(([cfg, info]) => {
        setConfig(cfg);
        setAppInfo(info);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const updateConfig = useCallback(
    async (partial: Partial<AppConfig>) => {
      const updated = { ...config, ...partial };
      setSaveStatus("saving");
      try {
        await invoke("set_app_config", { newConfig: updated });
        setConfig(updated);
        setSaveStatus("saved");
        setTimeout(() => setSaveStatus("idle"), 1500);
      } catch (err) {
        console.error("Failed to save config:", err);
        setSaveStatus("error");
        setTimeout(() => setSaveStatus("idle"), 3000);
      }
    },
    [config],
  );

  const updateNotifications = useCallback(
    async (partial: Partial<NotificationConfig>) => {
      await updateConfig({
        notifications: { ...config.notifications, ...partial },
      });
    },
    [config.notifications, updateConfig],
  );

  const setAutoStart = useCallback(async (enabled: boolean) => {
    try {
      await invoke("autostart_set", { enabled });
      setConfig((prev) => ({ ...prev, auto_start: enabled }));
    } catch (err) {
      console.error("Failed to set autostart:", err);
    }
  }, []);

  return {
    config,
    appInfo,
    loading,
    saveStatus,
    updateConfig,
    updateNotifications,
    setAutoStart,
  };
}
