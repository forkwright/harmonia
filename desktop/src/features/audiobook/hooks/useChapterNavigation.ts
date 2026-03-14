import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useChapterNavigation() {
  const nextChapter = useCallback(async () => {
    await invoke("audiobook_next_chapter");
  }, []);

  const prevChapter = useCallback(async () => {
    await invoke("audiobook_prev_chapter");
  }, []);

  const goToChapter = useCallback(async (chapter: number) => {
    await invoke("audiobook_go_to_chapter", { chapter });
  }, []);

  const skipForward = useCallback(async (seconds: number) => {
    await invoke("audiobook_skip_forward", { seconds });
  }, []);

  const skipBackward = useCallback(async (seconds: number) => {
    await invoke("audiobook_skip_backward", { seconds });
  }, []);

  return { nextChapter, prevChapter, goToChapter, skipForward, skipBackward };
}
