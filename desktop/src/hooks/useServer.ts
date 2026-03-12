import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ServerState {
  url: string;
  connected: boolean | null;
}

export function useServer(): ServerState {
  const [url, setUrl] = useState("");
  const [connected, setConnected] = useState<boolean | null>(null);

  useEffect(() => {
    invoke<string>("get_server_url").then((u) => {
      setUrl(u);
      invoke<boolean>("health_check", { serverUrl: u })
        .then(setConnected)
        .catch(() => setConnected(false));
    });
  }, []);

  return { url, connected };
}
