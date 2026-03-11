import type { ReactNode } from 'react'
import { Navigation } from './Navigation'
import { MiniPlayer } from './MiniPlayer'
import { usePlayerStore } from '../stores/playerStore'
import { usePodcastStore } from '../stores/podcastStore'
import { useLocation } from 'react-router-dom'

interface LayoutProps {
  readonly children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const currentEpisode = usePodcastStore((s) => s.currentEpisode)
  const location = useLocation()

  // Add bottom padding when mini-player is visible (has track, not on /player or /login)
  const hiddenPaths = ['/player', '/login']
  const hasMiniPlayer = (!!currentTrack || !!currentEpisode) && !hiddenPaths.includes(location.pathname)

  return (
    <div className="min-h-screen" style={{ backgroundColor: 'rgb(var(--surface-base))' }}>
      <Navigation />
      <main className={hasMiniPlayer ? 'pb-20' : ''}>
        {children}
      </main>
      <MiniPlayer />
    </div>
  )
}
