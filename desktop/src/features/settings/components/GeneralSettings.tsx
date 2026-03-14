import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig } from "../hooks/useSettings";

interface Props {
  config: AppConfig;
  onUpdate: (partial: Partial<AppConfig>) => Promise<void>;
}

export default function GeneralSettings({ config, onUpdate }: Props) {
  const [serverUrl, setServerUrl] = useState(config.server_url);
  const [checkStatus, setCheckStatus] = useState<"idle" | "checking" | "ok" | "error">("idle");

  async function handleSave() {
    await onUpdate({ server_url: serverUrl });
  }

  async function handleHealthCheck() {
    setCheckStatus("checking");
    try {
      const ok = await invoke<boolean>("health_check", { serverUrl });
      setCheckStatus(ok ? "ok" : "error");
    } catch {
      setCheckStatus("error");
    }
  }

  const statusLabel: Record<typeof checkStatus, string> = {
    idle: "",
    checking: "Checking…",
    ok: "Connected",
    error: "Unreachable",
  };

  const statusColor: Record<typeof checkStatus, string> = {
    idle: "",
    checking: "text-gray-400",
    ok: "text-green-400",
    error: "text-red-400",
  };

  return (
    <section className="space-y-4">
      <h2 className="text-base font-semibold text-gray-100">General</h2>

      <div className="space-y-2">
        <label className="block text-sm text-gray-300">Server URL</label>
        <div className="flex gap-2">
          <input
            className="flex-1 bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-100 focus:outline-none focus:border-indigo-500"
            value={serverUrl}
            onChange={(e) => setServerUrl(e.target.value)}
            placeholder="http://localhost:7700"
          />
          <button
            className="px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 rounded transition-colors"
            onClick={handleHealthCheck}
          >
            Test
          </button>
          <button
            className="px-3 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 rounded transition-colors"
            onClick={handleSave}
          >
            Save
          </button>
        </div>
        {checkStatus !== "idle" && (
          <p className={`text-xs ${statusColor[checkStatus]}`}>
            {statusLabel[checkStatus]}
          </p>
        )}
      </div>
    </section>
  );
}
