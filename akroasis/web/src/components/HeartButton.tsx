// Favorite toggle button — filled heart when favorited
import { useThymesisStore } from '../stores/thymesisStore'

interface HeartButtonProps {
  readonly trackId: number
  readonly size?: 'sm' | 'md'
}

export function HeartButton({ trackId, size = 'sm' }: HeartButtonProps) {
  const isFavorite = useThymesisStore((s) => s.isFavorite(trackId))
  const toggleFavorite = useThymesisStore((s) => s.toggleFavorite)

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation()
    void toggleFavorite(trackId)
  }

  const iconClass = size === 'md' ? 'w-5 h-5' : 'w-4 h-4'

  return (
    <button
      onClick={handleClick}
      className={`p-1 transition-colors ${
        isFavorite
          ? 'text-red-400 hover:text-red-300'
          : 'text-bronze-600 hover:text-red-400'
      }`}
      title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
      aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
    >
      <svg className={iconClass} viewBox="0 0 20 20" fill={isFavorite ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth={isFavorite ? 0 : 1.5}>
        <path fillRule="evenodd" d="M3.172 5.172a4 4 0 015.656 0L10 6.343l1.172-1.171a4 4 0 115.656 5.656L10 17.657l-6.828-6.829a4 4 0 010-5.656z" clipRule="evenodd" />
      </svg>
    </button>
  )
}
