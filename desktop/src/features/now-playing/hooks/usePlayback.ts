import { invoke } from "@tauri-apps/api/core";
import type { QueueEntry, RepeatMode } from "../../../types/playback";

export function usePlayback() {
  async function playTrack(
    entry: QueueEntry,
    baseUrl: string,
    token?: string
  ): Promise<void> {
    await invoke("play_track", { entry, baseUrl, token: token ?? null });
  }

  async function pause(): Promise<void> {
    await invoke("pause");
  }

  async function resume(): Promise<void> {
    await invoke("resume");
  }

  async function stop(): Promise<void> {
    await invoke("stop");
  }

  async function seek(positionMs: number): Promise<void> {
    await invoke("seek", { positionMs });
  }

  async function nextTrack(): Promise<void> {
    await invoke("next_track");
  }

  async function previousTrack(): Promise<void> {
    await invoke("previous_track");
  }

  async function setVolume(level: number): Promise<void> {
    await invoke("playback_set_volume", { level });
  }

  async function getVolume(): Promise<number> {
    return invoke<number>("playback_get_volume");
  }

  async function queueAdd(entries: QueueEntry[]): Promise<void> {
    await invoke("queue_add", { entries });
  }

  async function queueRemove(index: number): Promise<void> {
    await invoke("queue_remove", { index });
  }

  async function queueClear(): Promise<void> {
    await invoke("queue_clear");
  }

  async function queueMove(from: number, to: number): Promise<void> {
    await invoke("queue_move", { from, to });
  }

  async function setRepeatMode(mode: RepeatMode): Promise<void> {
    await invoke("set_repeat_mode", { mode });
  }

  async function setShuffle(enabled: boolean): Promise<void> {
    await invoke("set_shuffle", { enabled });
  }

  return {
    playTrack,
    pause,
    resume,
    stop,
    seek,
    nextTrack,
    previousTrack,
    setVolume,
    getVolume,
    queueAdd,
    queueRemove,
    queueClear,
    queueMove,
    setRepeatMode,
    setShuffle,
  };
}
