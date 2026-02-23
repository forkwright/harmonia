// Player page — routes to the correct player surface based on what's playing
import { usePlayerStore } from '../stores/playerStore'
import { usePodcastStore } from '../stores/podcastStore'
import { MusicPlayer } from '../components/MusicPlayer'
import { PodcastPlayer } from '../components/PodcastPlayer'

export function PlayerPage() {
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const currentEpisode = usePodcastStore((s) => s.currentEpisode)
  const currentShow = usePodcastStore((s) => s.currentShow)

  const isPodcast = !!currentEpisode && !!currentShow

  // Empty state
  if (!currentTrack && !isPodcast) {
    return (
      <div className="min-h-[calc(100vh-4rem)] flex flex-col items-center justify-center p-8">
        <div className="w-32 h-32 rounded-2xl bg-surface-raised flex items-center justify-center mb-6">
          <svg className="w-16 h-16 text-theme-muted" fill="currentColor" viewBox="0 0 20 20">
            <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
          </svg>
        </div>
        <p className="text-theme-tertiary text-lg">Nothing playing</p>
        <p className="text-theme-muted text-sm mt-1">Pick something from the library</p>
      </div>
    )
  }

  return (
    <div className="min-h-[calc(100vh-4rem)] flex items-start justify-center px-4 py-8">
      {isPodcast ? <PodcastPlayer /> : <MusicPlayer />}
    </div>
  )
}
