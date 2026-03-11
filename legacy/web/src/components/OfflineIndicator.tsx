import { useState, useEffect } from 'react'
import { scrobbleQueue } from '../services/scrobbleQueue'

export function OfflineIndicator() {
  const [isOnline, setIsOnline] = useState(navigator.onLine)
  const [pendingCount, setPendingCount] = useState(scrobbleQueue.queueSize())

  useEffect(() => {
    const handleOnline = () => {
      setIsOnline(true)
    }
    const handleOffline = () => setIsOnline(false)

    globalThis.addEventListener('online', handleOnline)
    globalThis.addEventListener('offline', handleOffline)

    return () => {
      globalThis.removeEventListener('online', handleOnline)
      globalThis.removeEventListener('offline', handleOffline)
    }
  }, [])

  useEffect(() => {
    const interval = setInterval(() => {
      setPendingCount(scrobbleQueue.queueSize())
    }, 3000)
    return () => clearInterval(interval)
  }, [])

  if (isOnline && pendingCount === 0) return null

  if (isOnline && pendingCount > 0) {
    return (
      <div
        role="status"
        aria-live="polite"
        style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          backgroundColor: '#3b82f6',
          color: '#fff',
          padding: '0.25rem 1rem',
          textAlign: 'center',
          fontSize: '0.8rem',
          fontWeight: '500',
          zIndex: 9999,
          boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        }}
      >
        Sending {pendingCount} queued scrobble{pendingCount !== 1 ? 's' : ''}...
      </div>
    )
  }

  return (
    <div
      role="alert"
      aria-live="assertive"
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        backgroundColor: '#f59e0b',
        color: '#000',
        padding: '0.5rem 1rem',
        textAlign: 'center',
        fontSize: '0.875rem',
        fontWeight: '500',
        zIndex: 9999,
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
      }}
    >
      Offline. Using cached content.
      {pendingCount > 0 && (
        <span style={{ marginLeft: '0.75rem', opacity: 0.8 }}>
          ({pendingCount} scrobble{pendingCount !== 1 ? 's' : ''} queued)
        </span>
      )}
    </div>
  )
}
