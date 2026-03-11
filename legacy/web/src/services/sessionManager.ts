// Playback session lifecycle management
import { apiClient } from '../api/client'
import { isTauri, randomUUID } from '../utils/platform'
import type { PlaybackSession } from '../types'

type MediaType = 'music' | 'audiobook' | 'podcast'

interface SessionConfig {
  mediaItemId: number
  mediaType: MediaType
  positionMs: number
  totalDurationMs: number
}

let activeSessionId: string | null = null

function getDeviceName(): string {
  if (isTauri()) return 'Akroasis Desktop'
  const ua = navigator.userAgent
  if (/Firefox/i.test(ua)) return 'Firefox'
  if (/Edg/i.test(ua)) return 'Edge'
  if (/Chrome/i.test(ua)) return 'Chrome'
  if (/Safari/i.test(ua)) return 'Safari'
  return 'Web Browser'
}

function getDeviceType(): string {
  if (isTauri()) return 'desktop'
  const hasTouch = 'ontouchstart' in globalThis || navigator.maxTouchPoints > 0
  if (hasTouch && globalThis.innerWidth < 768) return 'mobile'
  if (hasTouch) return 'tablet'
  return 'desktop'
}

async function startSession(config: SessionConfig): Promise<string> {
  const sessionId = randomUUID()
  activeSessionId = sessionId

  try {
    await apiClient.createSession({
      sessionId,
      mediaItemId: config.mediaItemId,
      userId: 'default',
      deviceName: getDeviceName(),
      deviceType: getDeviceType(),
      startedAt: new Date().toISOString(),
      startPositionMs: config.positionMs,
      durationMs: 0,
      isActive: true,
    })
  } catch {
    // Best-effort — don't block playback on session creation failure
  }

  return sessionId
}

async function updateSession(positionMs: number): Promise<void> {
  if (!activeSessionId) return

  try {
    await apiClient.updateSession(activeSessionId, {
      endPositionMs: positionMs,
      isActive: true,
    })
  } catch {
    // Best-effort
  }
}

async function endSession(positionMs: number): Promise<void> {
  if (!activeSessionId) return
  const sessionId = activeSessionId
  activeSessionId = null

  try {
    await apiClient.updateSession(sessionId, {
      endPositionMs: positionMs,
      endedAt: new Date().toISOString(),
      isActive: false,
    })
  } catch {
    // Best-effort
  }
}

async function getActiveSessions(): Promise<PlaybackSession[]> {
  try {
    const sessions = await apiClient.getSessions()
    return sessions.filter(
      (s) => s.isActive && s.sessionId !== activeSessionId,
    )
  } catch {
    return []
  }
}

function getActiveSessionId(): string | null {
  return activeSessionId
}

function reset(): void {
  activeSessionId = null
}

export const sessionManager = {
  startSession,
  updateSession,
  endSession,
  getActiveSessions,
  getActiveSessionId,
  getDeviceName,
  getDeviceType,
  reset,
}
