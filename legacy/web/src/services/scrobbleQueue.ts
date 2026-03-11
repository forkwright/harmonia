// Offline scrobble queue with auto-flush on reconnect
import { apiClient } from '../api/client'
import type { PendingScrobble } from '../types'

const STORAGE_KEY = 'akroasis_scrobble_queue'
const MAX_QUEUE_SIZE = 200
const MAX_ATTEMPTS = 3
const FLUSH_DELAY_MS = 1000

type ScrobbleInput = Omit<PendingScrobble, 'attempts'>

function readQueue(): PendingScrobble[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    return JSON.parse(raw) as PendingScrobble[]
  } catch {
    return []
  }
}

function writeQueue(queue: PendingScrobble[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(queue))
}

function enqueue(entry: PendingScrobble): void {
  const queue = readQueue()
  queue.push(entry)
  if (queue.length > MAX_QUEUE_SIZE) {
    queue.splice(0, queue.length - MAX_QUEUE_SIZE)
  }
  writeQueue(queue)
}

function dequeue(): PendingScrobble | undefined {
  const queue = readQueue()
  const item = queue.shift()
  writeQueue(queue)
  return item
}

function requeue(entry: PendingScrobble): void {
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
        await apiClient.scrobble({
          artist: item.artist,
          track: item.track,
          album: item.album,
          timestamp: item.timestamp,
          duration: item.duration,
        })
      } catch {
        const updatedItem: PendingScrobble = { ...item, attempts: item.attempts + 1 }
        if (updatedItem.attempts < MAX_ATTEMPTS) {
          requeue(updatedItem)
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

async function scrobble(input: ScrobbleInput): Promise<void> {
  if (!navigator.onLine) {
    enqueue({ ...input, attempts: 0 })
    return
  }

  try {
    await apiClient.scrobble(input)
  } catch {
    enqueue({ ...input, attempts: 0 })
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

export const scrobbleQueue = {
  scrobble,
  flush,
  enqueue,
  dequeue,
  requeue,
  queueSize,
  clearQueue,
  readQueue,
  init,
}
