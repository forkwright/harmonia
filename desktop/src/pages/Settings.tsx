import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function Settings() {
  const [serverUrl, setServerUrl] = useState("");
  const [status, setStatus] = useState<"idle" | "checking" | "ok" | "error">("idle");
  const [saveMessage, setSaveMessage] = useState("");

  useEffect(() => {
    invoke<string>("get_server_url").then(setServerUrl).catch(console.error);
  }, []);

  async function handleSave() {
    try {
      await invoke("set_server_url", { url: serverUrl });
      setSaveMessage("Saved.");
      setTimeout(() => setSaveMessage(""), 2000);
    } catch (err) {
      setSaveMessage(`Error: ${err}`);
    }
  }

  async function handleHealthCheck() {
    setStatus("checking");
    try {
      const ok = await invoke<boolean>("health_check", { serverUrl });
      setStatus(ok ? "ok" : "error");
    } catch {
      setStatus("error");
    }
  }

  const statusLabel: Record<typeof status, string> = {
    idle: "",
    checking: "Checking…",
    ok: "Connected",
    error: "Unreachable",
  };

  const statusColor: Record<typeof status, string> = {
    idle: "",
    checking: "text-gray-400",
    ok: "text-green-400",
    error: "text-red-400",
  };

  return (
    <div className="p-8 max-w-lg">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>
      <section className="space-y-4">
        <h2 className="text-base font-semibold text-gray-300">Server</h2>
        <div className="space-y-2">
          <label htmlFor="server-url" className="block text-sm text-gray-400">
            Server URL
          </label>
          <input
            id="server-url"
            type="url"
            value={serverUrl}
            onChange={(e) => setServerUrl(e.target.value)}
            placeholder="http://localhost:7700"
            className="w-full bg-gray-800 border border-gray-700 rounded-md px-3 py-2 text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
        <div className="flex gap-3 items-center">
          <button
            onClick={handleSave}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded-md transition-colors"
          >
            Save
          </button>
          <button
            onClick={handleHealthCheck}
            disabled={status === "checking"}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-white text-sm rounded-md transition-colors"
          >
            Check Connection
          </button>
          {saveMessage && <span className="text-sm text-gray-400">{saveMessage}</span>}
          {status !== "idle" && (
            <span className={`text-sm ${statusColor[status]}`}>{statusLabel[status]}</span>
          )}
        </div>
      </section>
    </div>
  );
}
