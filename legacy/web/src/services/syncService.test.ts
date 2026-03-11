import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { syncService } from './syncService'

vi.mock('../api/client', () => ({
  apiClient: {
    updateProgress: vi.fn().mockResolvedValue({}),
  },
}))

import { apiClient } from '../api/client'

const mockUpdateProgress = apiClient.updateProgress as ReturnType<typeof vi.fn>

const localStorageMap = new Map<string, string>()
const localStorageMock = {
  getItem: vi.fn((key: string) => localStorageMap.get(key) ?? null),
  setItem: vi.fn((key: string, value: string) => localStorageMap.set(key, value)),
  removeItem: vi.fn((key: string) => localStorageMap.delete(key)),
  clear: vi.fn(() => localStorageMap.clear()),
  get length() { return localStorageMap.size },
  key: vi.fn(),
}

Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock })

describe('syncService', () => {
  beforeEach(() => {
    localStorageMap.clear()
    vi.clearAllMocks()
    syncService.clearQueue()
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true, configurable: true })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('queue operations', () => {
    it('reads empty queue from empty storage', () => {
      expect(syncService.readQueue()).toEqual([])
    })

    it('enqueues and reads back', () => {
      syncService.enqueue({
        mediaItemId: 1,
        positionMs: 5000,
        totalDurationMs: 300000,
        isComplete: false,
        timestamp: Date.now(),
        attempts: 0,
      })

      expect(syncService.queueSize()).toBe(1)
      const queue = syncService.readQueue()
      expect(queue[0].mediaItemId).toBe(1)
    })

    it('deduplicates by mediaItemId on enqueue', () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 5000, totalDurationMs: 300000,
        isComplete: false, timestamp: 100, attempts: 0,
      })
      syncService.enqueue({
        mediaItemId: 1, positionMs: 10000, totalDurationMs: 300000,
        isComplete: false, timestamp: 200, attempts: 0,
      })

      expect(syncService.queueSize()).toBe(1)
      expect(syncService.readQueue()[0].positionMs).toBe(10000)
    })

    it('dequeues in FIFO order', () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })
      syncService.enqueue({
        mediaItemId: 2, positionMs: 2000, totalDurationMs: 200000,
        isComplete: false, timestamp: 200, attempts: 0,
      })

      const first = syncService.dequeue()
      expect(first?.mediaItemId).toBe(1)
      expect(syncService.queueSize()).toBe(1)
    })

    it('requeues to front', () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })
      const item = syncService.dequeue()!
      syncService.requeue({ ...item, attempts: 1 })

      const front = syncService.readQueue()[0]
      expect(front.mediaItemId).toBe(1)
      expect(front.attempts).toBe(1)
    })

    it('caps queue at MAX_QUEUE_SIZE', () => {
      for (let i = 0; i < 110; i++) {
        syncService.enqueue({
          mediaItemId: i, positionMs: i * 1000, totalDurationMs: 300000,
          isComplete: false, timestamp: i, attempts: 0,
        })
      }

      expect(syncService.queueSize()).toBe(100)
    })

    it('clears queue', () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })
      syncService.clearQueue()
      expect(syncService.queueSize()).toBe(0)
    })
  })

  describe('reportProgress', () => {
    it('calls API directly when online', async () => {
      await syncService.reportProgress(42, 5000, 300000)

      expect(mockUpdateProgress).toHaveBeenCalledWith(42, 5000, 300000, false)
      expect(syncService.queueSize()).toBe(0)
    })

    it('enqueues when offline', async () => {
      Object.defineProperty(navigator, 'onLine', { value: false })

      await syncService.reportProgress(42, 5000, 300000)

      expect(mockUpdateProgress).not.toHaveBeenCalled()
      expect(syncService.queueSize()).toBe(1)
    })

    it('enqueues on API failure', async () => {
      mockUpdateProgress.mockRejectedValueOnce(new Error('Network error'))

      await syncService.reportProgress(42, 5000, 300000)

      expect(syncService.queueSize()).toBe(1)
    })

    it('passes isComplete flag', async () => {
      await syncService.reportProgress(42, 300000, 300000, true)

      expect(mockUpdateProgress).toHaveBeenCalledWith(42, 300000, 300000, true)
    })
  })

  describe('flush', () => {
    it('drains queue when online', async () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })
      syncService.enqueue({
        mediaItemId: 2, positionMs: 2000, totalDurationMs: 200000,
        isComplete: false, timestamp: 200, attempts: 0,
      })

      await syncService.flush()

      expect(mockUpdateProgress).toHaveBeenCalledTimes(2)
      expect(syncService.queueSize()).toBe(0)
    })

    it('requeues failed items with incremented attempts', async () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })

      mockUpdateProgress.mockRejectedValueOnce(new Error('fail'))
      await syncService.flush()

      expect(syncService.queueSize()).toBe(1)
      expect(syncService.readQueue()[0].attempts).toBe(1)
    })

    it('drops items exceeding max attempts', async () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 2,
      })

      mockUpdateProgress.mockRejectedValueOnce(new Error('fail'))
      await syncService.flush()

      expect(syncService.queueSize()).toBe(0)
    })

    it('skips flush when offline', async () => {
      Object.defineProperty(navigator, 'onLine', { value: false })
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })

      await syncService.flush()

      expect(mockUpdateProgress).not.toHaveBeenCalled()
      expect(syncService.queueSize()).toBe(1)
    })
  })

  describe('startAutoSync', () => {
    it('syncs on interval', async () => {
      vi.useFakeTimers()
      const getState = vi.fn().mockReturnValue({
        mediaItemId: 1, positionMs: 5000, totalDurationMs: 300000,
      })

      const cleanup = syncService.startAutoSync(getState)

      await vi.advanceTimersByTimeAsync(30_000)
      expect(mockUpdateProgress).toHaveBeenCalledWith(1, 5000, 300000, false)

      cleanup()
      vi.useRealTimers()
    })

    it('skips sync when getState returns null', async () => {
      vi.useFakeTimers()
      const getState = vi.fn().mockReturnValue(null)

      const cleanup = syncService.startAutoSync(getState)

      await vi.advanceTimersByTimeAsync(30_000)
      expect(mockUpdateProgress).not.toHaveBeenCalled()

      cleanup()
      vi.useRealTimers()
    })

    it('syncs on visibilitychange hidden', () => {
      const getState = vi.fn().mockReturnValue({
        mediaItemId: 1, positionMs: 5000, totalDurationMs: 300000,
      })

      const cleanup = syncService.startAutoSync(getState)

      Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true })
      document.dispatchEvent(new Event('visibilitychange'))

      expect(getState).toHaveBeenCalled()

      Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true })
      cleanup()
    })

    it('cleanup removes all listeners', () => {
      vi.useFakeTimers()
      const getState = vi.fn().mockReturnValue(null)

      const cleanup = syncService.startAutoSync(getState)
      cleanup()

      getState.mockClear()
      vi.advanceTimersByTime(60_000)
      expect(getState).not.toHaveBeenCalled()

      vi.useRealTimers()
    })
  })

  describe('init', () => {
    it('flushes on online event', async () => {
      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })

      const cleanup = syncService.init()

      globalThis.dispatchEvent(new Event('online'))
      await vi.waitFor(() => expect(mockUpdateProgress).toHaveBeenCalled())

      cleanup()
    })

    it('cleanup removes online listener', () => {
      const cleanup = syncService.init()
      cleanup()

      syncService.enqueue({
        mediaItemId: 1, positionMs: 1000, totalDurationMs: 100000,
        isComplete: false, timestamp: 100, attempts: 0,
      })

      globalThis.dispatchEvent(new Event('online'))
      expect(mockUpdateProgress).not.toHaveBeenCalled()
    })
  })
})
