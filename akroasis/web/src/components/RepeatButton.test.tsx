// RepeatButton component tests
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'

const mockState = {
  repeatMode: 'off' as 'off' | 'all' | 'one' | 'shuffle-repeat',
  cycleRepeatMode: vi.fn(),
}

vi.mock('../stores/playerStore', () => ({
  usePlayerStore: vi.fn((sel?: (s: typeof mockState) => unknown) =>
    sel ? sel(mockState) : mockState,
  ),
}))

import { RepeatButton } from './RepeatButton'

describe('RepeatButton', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockState.repeatMode = 'off'
  })

  it('renders with repeat off label', () => {
    render(<RepeatButton />)
    expect(screen.getByRole('button', { name: 'Repeat off' })).toBeInTheDocument()
  })

  it('calls cycleRepeatMode on click', () => {
    render(<RepeatButton />)
    fireEvent.click(screen.getByRole('button'))
    expect(mockState.cycleRepeatMode).toHaveBeenCalledOnce()
  })

  it('shows active styling when mode is not off', () => {
    mockState.repeatMode = 'all'
    render(<RepeatButton />)
    const btn = screen.getByRole('button', { name: 'Repeat all' })
    expect(btn.className).toContain('text-bronze-100')
  })

  it('shows inactive styling when mode is off', () => {
    render(<RepeatButton />)
    const btn = screen.getByRole('button', { name: 'Repeat off' })
    expect(btn.className).toContain('text-bronze-500')
  })

  it('shows "1" badge for repeat one mode', () => {
    mockState.repeatMode = 'one'
    render(<RepeatButton />)
    expect(screen.getByText('1')).toBeInTheDocument()
  })

  it('does not show "1" badge for other modes', () => {
    mockState.repeatMode = 'all'
    render(<RepeatButton />)
    expect(screen.queryByText('1')).not.toBeInTheDocument()
  })

  it('renders correct label for shuffle-repeat', () => {
    mockState.repeatMode = 'shuffle-repeat'
    render(<RepeatButton />)
    expect(screen.getByRole('button', { name: 'Shuffle and repeat' })).toBeInTheDocument()
  })
})
