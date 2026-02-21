// Tests for offline scrobble queue
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import type { PendingScrobble } from '../types'

// --- localStorage mock ---

const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value },
    removeItem: (key: string) => { delete store[key] },
    clear: () => { store = {} },
  }
})()

Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock, writable: true })

// --- navigator.onLine mock ---

let onlineState = true
Object.defineProperty(globalThis.navigator, 'onLine', {
  get: () => onlineState,
  configurable: true,
})

// --- apiClient mock ---

vi.mock('../api/client', () => ({
  apiClient: {
    scrobble: vi.fn(),
  },
}))

import { apiClient } from '../api/client'
import { scrobbleQueue } from './scrobbleQueue'

const STORAGE_KEY = 'akroasis_scrobble_queue'

const mockScrobble = apiClient.scrobble as ReturnType<typeof vi.fn>

function makeEntry(overrides: Partial<PendingScrobble> = {}): PendingScrobble {
  return {
    artist: 'Artist',
    track: 'Track',
    album: 'Album',
    timestamp: 1700000000,
    duration: 180,
    attempts: 0,
    ...overrides,
  }
}

beforeEach(() => {
  localStorageMock.clear()
  onlineState = true
  vi.clearAllMocks()
})

afterEach(() => {
  localStorageMock.clear()
})

describe('scrobbleQueue.readQueue', () => {
  it('returns empty array when storage is empty', () => {
    expect(scrobbleQueue.readQueue()).toEqual([])
  })

  it('returns parsed queue from storage', () => {
    const entry = makeEntry()
    localStorageMock.setItem(STORAGE_KEY, JSON.stringify([entry]))
    expect(scrobbleQueue.readQueue()).toEqual([entry])
  })

  it('returns empty array on corrupted storage', () => {
    localStorageMock.setItem(STORAGE_KEY, '{bad json')
    expect(scrobbleQueue.readQueue()).toEqual([])
  })
})

describe('scrobbleQueue.enqueue', () => {
  it('adds an item to the queue', () => {
    const entry = makeEntry()
    scrobbleQueue.enqueue(entry)
    expect(scrobbleQueue.readQueue()).toEqual([entry])
  })

  it('appends multiple items in order', () => {
    const a = makeEntry({ track: 'A' })
    const b = makeEntry({ track: 'B' })
    scrobbleQueue.enqueue(a)
    scrobbleQueue.enqueue(b)
    const queue = scrobbleQueue.readQueue()
    expect(queue[0].track).toBe('A')
    expect(queue[1].track).toBe('B')
  })

  it('trims oldest entries when MAX_QUEUE_SIZE is exceeded', () => {
    for (let i = 0; i < 201; i++) {
      scrobbleQueue.enqueue(makeEntry({ track: `Track ${i}`, timestamp: i }))
    }
    const queue = scrobbleQueue.readQueue()
    expect(queue.length).toBe(200)
    expect(queue[0].track).toBe('Track 1')
    expect(queue[199].track).toBe('Track 200')
  })
})

describe('scrobbleQueue.dequeue', () => {
  it('returns undefined when queue is empty', () => {
    expect(scrobbleQueue.dequeue()).toBeUndefined()
  })

  it('removes and returns the first item', () => {
    const a = makeEntry({ track: 'A' })
    const b = makeEntry({ track: 'B' })
    scrobbleQueue.enqueue(a)
    scrobbleQueue.enqueue(b)
    const item = scrobbleQueue.dequeue()
    expect(item?.track).toBe('A')
    expect(scrobbleQueue.queueSize()).toBe(1)
  })
})

describe('scrobbleQueue.requeue', () => {
  it('puts an item at the front of the queue', () => {
    const a = makeEntry({ track: 'A' })
    const b = makeEntry({ track: 'B' })
    scrobbleQueue.enqueue(b)
    scrobbleQueue.requeue(a)
    expect(scrobbleQueue.readQueue()[0].track).toBe('A')
  })
})

describe('scrobbleQueue.queueSize', () => {
  it('returns 0 for empty queue', () => {
    expect(scrobbleQueue.queueSize()).toBe(0)
  })

  it('returns correct count', () => {
    scrobbleQueue.enqueue(makeEntry())
    scrobbleQueue.enqueue(makeEntry())
    expect(scrobbleQueue.queueSize()).toBe(2)
  })
})

describe('scrobbleQueue.clearQueue', () => {
  it('removes all items and the storage key', () => {
    scrobbleQueue.enqueue(makeEntry())
    scrobbleQueue.clearQueue()
    expect(scrobbleQueue.queueSize()).toBe(0)
    expect(localStorageMock.getItem(STORAGE_KEY)).toBeNull()
  })
})

describe('scrobbleQueue.scrobble — online', () => {
  it('calls apiClient.scrobble directly when online', async () => {
    mockScrobble.mockResolvedValueOnce(undefined)
    await scrobbleQueue.scrobble(makeEntry())
    expect(mockScrobble).toHaveBeenCalledOnce()
    expect(scrobbleQueue.queueSize()).toBe(0)
  })

  it('enqueues with attempts=0 when apiClient.scrobble throws', async () => {
    mockScrobble.mockRejectedValueOnce(new Error('network'))
    await scrobbleQueue.scrobble(makeEntry({ track: 'Failed' }))
    expect(scrobbleQueue.queueSize()).toBe(1)
    expect(scrobbleQueue.readQueue()[0].attempts).toBe(0)
    expect(scrobbleQueue.readQueue()[0].track).toBe('Failed')
  })
})

describe('scrobbleQueue.scrobble — offline', () => {
  it('enqueues without calling API when offline', async () => {
    onlineState = false
    await scrobbleQueue.scrobble(makeEntry({ track: 'Offline Track' }))
    expect(mockScrobble).not.toHaveBeenCalled()
    expect(scrobbleQueue.queueSize()).toBe(1)
    expect(scrobbleQueue.readQueue()[0].track).toBe('Offline Track')
  })

  it('sets attempts=0 on newly queued offline entries', async () => {
    onlineState = false
    await scrobbleQueue.scrobble(makeEntry())
    expect(scrobbleQueue.readQueue()[0].attempts).toBe(0)
  })
})

describe('scrobbleQueue.flush', () => {
  it('does nothing when offline', async () => {
    onlineState = false
    scrobbleQueue.enqueue(makeEntry())
    await scrobbleQueue.flush()
    expect(mockScrobble).not.toHaveBeenCalled()
    expect(scrobbleQueue.queueSize()).toBe(1)
  })

  it('sends all queued items when online', async () => {
    mockScrobble.mockResolvedValue(undefined)
    scrobbleQueue.enqueue(makeEntry({ track: 'A' }))
    scrobbleQueue.enqueue(makeEntry({ track: 'B' }))

    // Patch delay to resolve immediately so test doesn't take 1s per item
    vi.useFakeTimers()
    const flushPromise = scrobbleQueue.flush()
    await vi.runAllTimersAsync()
    await flushPromise
    vi.useRealTimers()

    expect(mockScrobble).toHaveBeenCalledTimes(2)
    expect(scrobbleQueue.queueSize()).toBe(0)
  })

  it('requeues failed item and stops flush', async () => {
    mockScrobble.mockRejectedValueOnce(new Error('timeout'))
    scrobbleQueue.enqueue(makeEntry({ track: 'Fail', attempts: 0 }))
    scrobbleQueue.enqueue(makeEntry({ track: 'Next', attempts: 0 }))

    await scrobbleQueue.flush()

    // Failed item incremented attempts and put back first, Next not yet processed
    const queue = scrobbleQueue.readQueue()
    expect(queue[0].track).toBe('Fail')
    expect(queue[0].attempts).toBe(1)
    expect(queue[1].track).toBe('Next')
  })

  it('discards item after MAX_ATTEMPTS failures', async () => {
    mockScrobble.mockRejectedValue(new Error('permanent'))
    scrobbleQueue.enqueue(makeEntry({ attempts: 2 }))

    await scrobbleQueue.flush()

    expect(scrobbleQueue.queueSize()).toBe(0)
  })

  it('does nothing when queue is empty', async () => {
    await scrobbleQueue.flush()
    expect(mockScrobble).not.toHaveBeenCalled()
  })
})

describe('scrobbleQueue.init', () => {
  it('returns a cleanup function', () => {
    const cleanup = scrobbleQueue.init()
    expect(typeof cleanup).toBe('function')
    cleanup()
  })

  it('triggers flush when online event fires', async () => {
    onlineState = false
    mockScrobble.mockResolvedValue(undefined)
    scrobbleQueue.enqueue(makeEntry({ track: 'Queued' }))

    const cleanup = scrobbleQueue.init()

    onlineState = true
    vi.useFakeTimers()
    globalThis.dispatchEvent(new Event('online'))
    await vi.runAllTimersAsync()
    vi.useRealTimers()

    // Give microtasks time to settle
    await Promise.resolve()

    cleanup()

    expect(mockScrobble).toHaveBeenCalledTimes(1)
    expect(scrobbleQueue.queueSize()).toBe(0)
  })
})
