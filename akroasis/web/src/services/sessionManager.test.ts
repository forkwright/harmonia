import { describe, it, expect, vi, beforeEach } from 'vitest'
import { sessionManager } from './sessionManager'

vi.mock('../api/client', () => ({
  apiClient: {
    createSession: vi.fn().mockResolvedValue({}),
    updateSession: vi.fn().mockResolvedValue({}),
    getSessions: vi.fn().mockResolvedValue([]),
  },
}))

vi.mock('../utils/platform', () => ({
  isTauri: vi.fn().mockReturnValue(false),
}))

import { apiClient } from '../api/client'

const mockCreateSession = apiClient.createSession as ReturnType<typeof vi.fn>
const mockUpdateSession = apiClient.updateSession as ReturnType<typeof vi.fn>
const mockGetSessions = apiClient.getSessions as ReturnType<typeof vi.fn>

describe('sessionManager', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    sessionManager.reset()
  })

  describe('device detection', () => {
    it('returns a valid device name', () => {
      const name = sessionManager.getDeviceName()
      expect(typeof name).toBe('string')
      expect(name.length).toBeGreaterThan(0)
    })

    it('returns a valid device type', () => {
      const type = sessionManager.getDeviceType()
      expect(['desktop', 'mobile', 'tablet']).toContain(type)
    })
  })

  describe('startSession', () => {
    it('creates session via API and returns sessionId', async () => {
      const id = await sessionManager.startSession({
        mediaItemId: 42,
        mediaType: 'music',
        positionMs: 0,
        totalDurationMs: 300000,
      })

      expect(typeof id).toBe('string')
      expect(id.length).toBeGreaterThan(0)
      expect(mockCreateSession).toHaveBeenCalledTimes(1)

      const call = mockCreateSession.mock.calls[0][0]
      expect(call.mediaItemId).toBe(42)
      expect(call.sessionId).toBe(id)
      expect(call.isActive).toBe(true)
      expect(call.startPositionMs).toBe(0)
    })

    it('stores activeSessionId', async () => {
      const id = await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'audiobook', positionMs: 5000, totalDurationMs: 600000,
      })

      expect(sessionManager.getActiveSessionId()).toBe(id)
    })

    it('does not throw on API failure', async () => {
      mockCreateSession.mockRejectedValueOnce(new Error('Network error'))

      const id = await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'music', positionMs: 0, totalDurationMs: 300000,
      })

      expect(typeof id).toBe('string')
    })
  })

  describe('updateSession', () => {
    it('updates position via API', async () => {
      const id = await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'music', positionMs: 0, totalDurationMs: 300000,
      })

      await sessionManager.updateSession(15000)

      expect(mockUpdateSession).toHaveBeenCalledWith(id, {
        endPositionMs: 15000,
        isActive: true,
      })
    })

    it('skips when no active session', async () => {
      await sessionManager.updateSession(15000)
      expect(mockUpdateSession).not.toHaveBeenCalled()
    })

    it('does not throw on API failure', async () => {
      await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'music', positionMs: 0, totalDurationMs: 300000,
      })

      mockUpdateSession.mockRejectedValueOnce(new Error('fail'))
      await expect(sessionManager.updateSession(15000)).resolves.toBeUndefined()
    })
  })

  describe('endSession', () => {
    it('marks session inactive and clears activeSessionId', async () => {
      const id = await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'music', positionMs: 0, totalDurationMs: 300000,
      })

      await sessionManager.endSession(150000)

      expect(mockUpdateSession).toHaveBeenCalledWith(id, expect.objectContaining({
        endPositionMs: 150000,
        isActive: false,
      }))
      expect(sessionManager.getActiveSessionId()).toBeNull()
    })

    it('skips when no active session', async () => {
      await sessionManager.endSession(0)
      expect(mockUpdateSession).not.toHaveBeenCalled()
    })
  })

  describe('getActiveSessions', () => {
    it('returns active sessions excluding self', async () => {
      const selfId = await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'music', positionMs: 0, totalDurationMs: 300000,
      })

      mockGetSessions.mockResolvedValueOnce([
        { sessionId: selfId, isActive: true, mediaItemId: 1 },
        { sessionId: 'other-device', isActive: true, mediaItemId: 2 },
        { sessionId: 'ended', isActive: false, mediaItemId: 3 },
      ])

      const active = await sessionManager.getActiveSessions()

      expect(active).toHaveLength(1)
      expect(active[0].sessionId).toBe('other-device')
    })

    it('returns empty array on API failure', async () => {
      mockGetSessions.mockRejectedValueOnce(new Error('fail'))

      const active = await sessionManager.getActiveSessions()
      expect(active).toEqual([])
    })
  })

  describe('reset', () => {
    it('clears activeSessionId', async () => {
      await sessionManager.startSession({
        mediaItemId: 1, mediaType: 'music', positionMs: 0, totalDurationMs: 300000,
      })

      sessionManager.reset()
      expect(sessionManager.getActiveSessionId()).toBeNull()
    })
  })
})
