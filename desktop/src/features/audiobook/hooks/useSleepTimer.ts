import { useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

interface SleepTimerState {
  endOfChapter: boolean;
  totalSecs: number;
  elapsedSecs: number;
  remainingSecs: number;
  fading: boolean;
}

const POLL_INTERVAL_MS = 1000;
const SLEEP_TIMER_QUERY_KEY = ["sleep-timer"] as const;

async function fetchTimerState(): Promise<SleepTimerState | null> {
  return invoke<SleepTimerState | null>("sleep_timer_get");
}

export function useSleepTimer() {
  const queryClient = useQueryClient();

  const { data: timerState } = useQuery({
    queryKey: SLEEP_TIMER_QUERY_KEY,
    queryFn: fetchTimerState,
    refetchInterval: POLL_INTERVAL_MS,
  });

  const invalidate = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: SLEEP_TIMER_QUERY_KEY });
  }, [queryClient]);

  const setTimer = useCallback(
    async (minutes: number) => {
      await invoke("sleep_timer_set", { minutes });
      await invalidate();
    },
    [invalidate]
  );

  const setEndOfChapter = useCallback(async () => {
    await invoke("sleep_timer_set_end_of_chapter");
    await invalidate();
  }, [invalidate]);

  const cancel = useCallback(async () => {
    await invoke("sleep_timer_cancel");
    await invalidate();
  }, [invalidate]);

  const extendFiveMinutes = useCallback(async () => {
    await invoke("sleep_timer_extend", { minutes: 5 });
    await invalidate();
  }, [invalidate]);

  return {
    timerState: timerState ?? null,
    isActive: timerState != null,
    setTimer,
    setEndOfChapter,
    cancel,
    extendFiveMinutes,
  };
}
