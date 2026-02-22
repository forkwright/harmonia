// Cross-device playback transfer banner
import { useState, useEffect } from 'react'
import { sessionManager } from '../services/sessionManager'
import type { PlaybackSession } from '../types'

interface PlaybackTransferProps {
  onTransfer: (session: PlaybackSession) => void
}

function formatDurationMs(ms: number): string {
  const totalMinutes = Math.floor(ms / 60000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (hours === 0) return `${minutes}m`
  return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`
}

export function PlaybackTransfer({ onTransfer }: PlaybackTransferProps) {
  const [sessions, setSessions] = useState<PlaybackSession[]>([])
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    let cancelled = false
    sessionManager.getActiveSessions().then((active) => {
      if (!cancelled && active.length > 0) {
        setSessions(active)
        setVisible(true)
      }
    })
    return () => { cancelled = true }
  }, [])

  if (!visible || sessions.length === 0) return null

  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 bg-bronze-800 border border-bronze-600 rounded-xl p-4 shadow-2xl z-50 max-w-md w-[calc(100%-2rem)]">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-bronze-200">Playing elsewhere</h3>
        <button
          onClick={() => setVisible(false)}
          className="text-bronze-500 hover:text-bronze-300 transition-colors"
          aria-label="Dismiss"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <div className="space-y-2">
        {sessions.map((session) => (
          <button
            key={session.sessionId}
            onClick={() => {
              onTransfer(session)
              setVisible(false)
            }}
            className="w-full text-left p-3 bg-bronze-700/50 rounded-lg hover:bg-bronze-700 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="text-bronze-100 text-sm font-medium">{session.deviceName}</p>
                <p className="text-bronze-400 text-xs mt-0.5">
                  {session.deviceType} &middot; {formatDurationMs(session.durationMs)} listened
                </p>
              </div>
              <span className="text-bronze-400 text-xs">Continue here →</span>
            </div>
          </button>
        ))}
      </div>
    </div>
  )
}
