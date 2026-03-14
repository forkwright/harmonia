import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { QueueChangedEvent, QueueState } from "../../../types/playback";

const INITIAL_QUEUE: QueueState = {
  entries: [],
  current_index: 0,
  repeat_mode: "off",
  shuffle: false,
  source_label: "",
};

export function useQueue(): QueueState {
  const [queue, setQueue] = useState<QueueState>(INITIAL_QUEUE);

  useEffect(() => {
    invoke<QueueState>("queue_get")
      .then((q) => setQueue(q))
      .catch(() => {});

    const unlisten = listen<QueueChangedEvent>("queue-changed", (event) => {
      setQueue((prev) => ({
        ...prev,
        entries: event.payload.entries,
        current_index: event.payload.current_index,
      }));
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return queue;
}
