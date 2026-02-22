// HeartButton component tests
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'

const mockToggleFavorite = vi.fn()
let mockIsFavoriteResult = false

const mockState = {
  isFavorite: (_id: number) => mockIsFavoriteResult,
  toggleFavorite: mockToggleFavorite,
}

vi.mock('../stores/thymesisStore', () => ({
  useThymesisStore: vi.fn((sel?: (s: typeof mockState) => unknown) =>
    sel ? sel(mockState) : mockState,
  ),
}))

import { HeartButton } from './HeartButton'

describe('HeartButton', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockIsFavoriteResult = false
  })

  it('renders unfavorited state', () => {
    render(<HeartButton trackId={1} />)
    expect(screen.getByRole('button', { name: 'Add to favorites' })).toBeInTheDocument()
  })

  it('renders favorited state', () => {
    mockIsFavoriteResult = true
    render(<HeartButton trackId={1} />)
    expect(screen.getByRole('button', { name: 'Remove from favorites' })).toBeInTheDocument()
  })

  it('calls toggleFavorite on click', () => {
    render(<HeartButton trackId={42} />)
    fireEvent.click(screen.getByRole('button'))
    expect(mockToggleFavorite).toHaveBeenCalledWith(42)
  })

  it('stops event propagation', () => {
    const parentHandler = vi.fn()
    render(
      <div onClick={parentHandler}>
        <HeartButton trackId={1} />
      </div>
    )
    fireEvent.click(screen.getByRole('button'))
    expect(parentHandler).not.toHaveBeenCalled()
  })

  it('applies red color when favorited', () => {
    mockIsFavoriteResult = true
    render(<HeartButton trackId={1} />)
    expect(screen.getByRole('button').className).toContain('text-red-400')
  })

  it('applies bronze color when not favorited', () => {
    render(<HeartButton trackId={1} />)
    expect(screen.getByRole('button').className).toContain('text-bronze-600')
  })
})
