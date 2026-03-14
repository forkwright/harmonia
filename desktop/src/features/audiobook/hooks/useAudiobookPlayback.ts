import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useLibraryStore } from "../../library/store";
import { useAudiobookPlayerStore } from "../store";
import type { Chapter } from "../../../types/media";

interface TauriChapterInfo {
  position: number;
  title: string;
  startMs: number;
  endMs: number;
}

function toTauriChapters(chapters: Chapter[]): TauriChapterInfo[] {
  return chapters.map((c) => ({
    position: c.position,
    title: c.title,
    startMs: c.startMs,
    endMs: c.endMs,
  }));
}

export function useAudiobookPlayback() {
  const token = useLibraryStore((s) => s.token);
  const recordListened = useAudiobookPlayerStore((s) => s.recordListened);
  const storedSpeed = useAudiobookPlayerStore((s) => s.speed);
  const setStoredSpeed = useAudiobookPlayerStore((s) => s.setSpeed);

  const play = useCallback(
    async (audiobookId: string, chapters: Chapter[]) => {
      const serverUrl: string = await invoke("get_server_url");
      await invoke("audiobook_play", {
        audiobookId,
        chapters: toTauriChapters(chapters),
        serverUrl,
        token,
      });
      const speed = storedSpeed(audiobookId);
      if (speed !== 1.0) {
        await invoke("audiobook_set_speed", { speed });
      }
      recordListened(audiobookId);
    },
    [token, storedSpeed, recordListened]
  );

  const playFromChapter = useCallback(
    async (audiobookId: string, chapter: number, chapters: Chapter[]) => {
      const serverUrl: string = await invoke("get_server_url");
      await invoke("audiobook_play_from_chapter", {
        audiobookId,
        chapter,
        chapters: toTauriChapters(chapters),
        serverUrl,
        token,
      });
      const speed = storedSpeed(audiobookId);
      if (speed !== 1.0) {
        await invoke("audiobook_set_speed", { speed });
      }
      recordListened(audiobookId);
    },
    [token, storedSpeed, recordListened]
  );

  const resume = useCallback(
    async (
      audiobookId: string,
      chapter: number,
      offsetMs: number,
      chapters: Chapter[]
    ) => {
      const serverUrl: string = await invoke("get_server_url");
      await invoke("audiobook_resume", {
        audiobookId,
        chapter,
        offsetMs,
        chapters: toTauriChapters(chapters),
        serverUrl,
        token,
      });
      const speed = storedSpeed(audiobookId);
      if (speed !== 1.0) {
        await invoke("audiobook_set_speed", { speed });
      }
      recordListened(audiobookId);
    },
    [token, storedSpeed, recordListened]
  );

  const pause = useCallback(async () => {
    await invoke("audiobook_pause");
  }, []);

  const stop = useCallback(async () => {
    await invoke("audiobook_stop");
  }, []);

  const setSpeed = useCallback(
    async (audiobookId: string, speed: number) => {
      await invoke("audiobook_set_speed", { speed });
      setStoredSpeed(audiobookId, speed);
    },
    [setStoredSpeed]
  );

  return { play, playFromChapter, resume, pause, stop, setSpeed };
}
