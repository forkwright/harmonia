import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { Navigation } from './Navigation'

const mockLogout = vi.fn()
const mockAuthState = { logout: mockLogout }

vi.mock('../stores/authStore', () => ({
  useAuthStore: vi.fn((sel?: (s: typeof mockAuthState) => unknown) =>
    sel ? sel(mockAuthState) : mockAuthState,
  ),
}))

const mockSearchState = {
  query: '',
  results: [],
  isOpen: false,
  selectedIndex: -1,
  setQuery: vi.fn(),
  search: vi.fn(),
  setOpen: vi.fn(),
  setSelectedIndex: vi.fn(),
  clear: vi.fn(),
}

vi.mock('../stores/searchStore', () => ({
  useSearchStore: vi.fn((sel?: (s: typeof mockSearchState) => unknown) =>
    sel ? sel(mockSearchState) : mockSearchState,
  ),
}))

const mockPlayerState = { setCurrentTrack: vi.fn(), setIsPlaying: vi.fn() }

vi.mock('../stores/playerStore', () => ({
  usePlayerStore: vi.fn((sel?: (s: typeof mockPlayerState) => unknown) =>
    sel ? sel(mockPlayerState) : mockPlayerState,
  ),
}))

vi.mock('../hooks/useDebounce', () => ({
  useDebounce: (val: string) => val,
}))

function renderNav(path = '/library') {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Navigation />
    </MemoryRouter>,
  )
}

describe('Navigation', () => {
  beforeEach(() => vi.clearAllMocks())

  it('renders brand name', () => {
    renderNav()
    expect(screen.getByText('Akroasis')).toBeInTheDocument()
  })

  it('renders search input with combobox role', () => {
    renderNav()
    expect(screen.getByRole('combobox')).toBeInTheDocument()
  })

  it('renders all nav links', () => {
    renderNav()
    expect(screen.getAllByText('Music').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Books').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Podcasts').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Discover').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Queue').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Player').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Settings').length).toBeGreaterThan(0)
  })

  it('renders logout button', () => {
    renderNav()
    expect(screen.getAllByText('Logout').length).toBeGreaterThan(0)
  })

  it('renders hamburger menu button on mobile', () => {
    renderNav()
    expect(screen.getByLabelText('Open menu')).toBeInTheDocument()
  })

  it('shows nav items when hamburger is clicked', () => {
    renderNav()
    fireEvent.click(screen.getByLabelText('Open menu'))
    // Mobile menu duplicates nav items — check they're present
    expect(screen.getAllByText('Music').length).toBeGreaterThanOrEqual(2)
    expect(screen.getAllByText('Logout').length).toBeGreaterThanOrEqual(2)
  })

  it('closes mobile menu on close button click', () => {
    renderNav()
    fireEvent.click(screen.getByLabelText('Open menu'))
    expect(screen.getByLabelText('Close menu')).toBeInTheDocument()
    fireEvent.click(screen.getByLabelText('Close menu'))
    expect(screen.getByLabelText('Open menu')).toBeInTheDocument()
  })
})
