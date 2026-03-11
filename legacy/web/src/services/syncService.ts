// Offline-resilient progress sync with auto-flush on reconnect
import { apiClient } from '../api/client'

interface PendingProgressUpdate {
  mediaItemId: number
  positionMs: number
  totalDurationMs: number
  isComplete: boolean
  timestamp: number
  attempts: number
}

const STORAGE_KEY = 'akroasis_progress_queue'
const MAX_QUEUE_SIZE = 100
const MAX_ATTEMPTS = 3
const FLUSH_DELAY_MS = 500
const SYNC_INTERVAL_MS = 30_000

function readQueue(): PendingProgressUpdate[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    return JSON.parse(raw) as PendingProgressUpdate[]
  } catch {
    return []
  }
}

function writeQueue(queue: PendingProgressUpdate[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(queue))
}

function enqueue(entry: PendingProgressUpdate): void {
  const queue = readQueue().filter((e) => e.mediaItemId !== entry.mediaItemId)
  queue.push(entry)
  if (queue.length > MAX_QUEUE_SIZE) {
    queue.splice(0, queue.length - MAX_QUEUE_SIZE)
  }
  writeQueue(queue)
}

function dequeue(): PendingProgressUpdate | undefined {
  const queue = readQueue()
  const item = queue.shift()
  writeQueue(queue)
  return item
}

function requeue(entry: PendingProgressUpdate): void {
  const queue = readQueue()
  queue.unshift(entry)
  writeQueue(queue)
}

function queueSize(): number {
  return readQueue().length
}

function clearQueue(): void {
  localStorage.removeItem(STORAGE_KEY)
}

let flushing = false

async function flush(): Promise<void> {
  if (flushing || !navigator.onLine) return
  flushing = true

  try {
    while (navigator.onLine) {
      const item = dequeue()
      if (!item) break

      try {
        await apiClient.updateProgress(
          item.mediaItemId,
          item.positionMs,
          item.totalDurationMs,
          item.isComplete,
        )
      } catch {
        const updated: PendingProgressUpdate = { ...item, attempts: item.attempts + 1 }
        if (updated.attempts < MAX_ATTEMPTS) {
          requeue(updated)
        }
        break
      }

      if (queueSize() > 0) {
        await new Promise<void>((resolve) => setTimeout(resolve, FLUSH_DELAY_MS))
      }
    }
  } finally {
    flushing = false
  }
}

async function reportProgress(
  mediaItemId: number,
  positionMs: number,
  totalDurationMs: number,
  isComplete = false,
): Promise<void> {
  const entry: PendingProgressUpdate = {
    mediaItemId,
    positionMs,
    totalDurationMs,
    isComplete,
    timestamp: Date.now(),
    attempts: 0,
  }

  if (!navigator.onLine) {
    enqueue(entry)
    return
  }

  try {
    await apiClient.updateProgress(mediaItemId, positionMs, totalDurationMs, isComplete)
  } catch {
    enqueue(entry)
  }
}

type StateGetter = () => {
  mediaItemId: number
  positionMs: number
  totalDurationMs: number
} | null

function startAutoSync(getState: StateGetter): () => void {
  const syncNow = () => {
    const state = getState()
    if (state) {
      void reportProgress(state.mediaItemId, state.positionMs, state.totalDurationMs)
    }
  }

  const intervalId = setInterval(syncNow, SYNC_INTERVAL_MS)

  const handleVisibilityChange = () => {
    if (document.visibilityState === 'hidden') syncNow()
  }

  const handleBeforeUnload = () => syncNow()

  document.addEventListener('visibilitychange', handleVisibilityChange)
  globalThis.addEventListener('beforeunload', handleBeforeUnload)

  return () => {
    clearInterval(intervalId)
    document.removeEventListener('visibilitychange', handleVisibilityChange)
    globalThis.removeEventListener('beforeunload', handleBeforeUnload)
  }
}

function init(): () => void {
  const handleOnline = () => {
    void flush()
  }

  globalThis.addEventListener('online', handleOnline)

  return () => {
    globalThis.removeEventListener('online', handleOnline)
  }
}

export const syncService = {
  reportProgress,
  startAutoSync,
  flush,
  init,
  enqueue,
  dequeue,
  requeue,
  readQueue,
  queueSize,
  clearQueue,
}
