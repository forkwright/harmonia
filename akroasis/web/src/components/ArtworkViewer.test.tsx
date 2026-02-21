import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ArtworkViewer } from './ArtworkViewer'
import { useArtworkViewer } from '../stores/artworkViewerStore'

function openViewer(url = 'http://example.com/cover.jpg') {
  useArtworkViewer.getState().open(url)
}

function closeViewer() {
  useArtworkViewer.getState().close()
}

describe('ArtworkViewer', () => {
  beforeEach(() => {
    closeViewer()
  })

  describe('visibility', () => {
    it('renders nothing when closed', () => {
      render(<ArtworkViewer />)
      expect(screen.queryByTestId('artwork-viewer')).toBeNull()
    })

    it('renders when open', () => {
      openViewer()
      render(<ArtworkViewer />)
      expect(screen.getByTestId('artwork-viewer')).toBeInTheDocument()
    })

    it('shows the artwork image with the correct url', () => {
      const url = 'http://example.com/cover.jpg'
      openViewer(url)
      render(<ArtworkViewer />)
      const img = screen.getByRole('img', { name: 'Album artwork' })
      expect(img).toHaveAttribute('src', url)
    })
  })

  describe('closing', () => {
    it('closes on Escape key', () => {
      openViewer()
      render(<ArtworkViewer />)
      expect(screen.getByTestId('artwork-viewer')).toBeInTheDocument()

      fireEvent.keyDown(document, { key: 'Escape' })
      expect(useArtworkViewer.getState().isOpen).toBe(false)
    })

    it('closes when X button is clicked', () => {
      openViewer()
      render(<ArtworkViewer />)

      fireEvent.click(screen.getByRole('button', { name: 'Close artwork viewer' }))
      expect(useArtworkViewer.getState().isOpen).toBe(false)
    })

    it('closes when backdrop is clicked', () => {
      openViewer()
      render(<ArtworkViewer />)

      fireEvent.click(screen.getByTestId('artwork-backdrop'))
      expect(useArtworkViewer.getState().isOpen).toBe(false)
    })
  })

  describe('zoom', () => {
    it('increases scale on scroll-up (wheel zoom in)', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')
      fireEvent.wheel(container, { deltaY: -100 })

      const img = screen.getByRole('img', { name: 'Album artwork' })
      const transform = (img as HTMLImageElement).style.transform
      expect(transform).toContain('scale(1.25)')
    })

    it('decreases scale on scroll-down (wheel zoom out)', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')
      // Zoom in first so we have room to zoom out
      fireEvent.wheel(container, { deltaY: -100 })
      fireEvent.wheel(container, { deltaY: 100 })

      const img = screen.getByRole('img', { name: 'Album artwork' })
      const transform = (img as HTMLImageElement).style.transform
      expect(transform).toContain('scale(1)')
    })

    it('clamps scale at maximum (5x)', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')
      // Scroll up many times beyond max
      for (let i = 0; i < 30; i++) {
        fireEvent.wheel(container, { deltaY: -100 })
      }

      const img = screen.getByRole('img', { name: 'Album artwork' })
      expect((img as HTMLImageElement).style.transform).toContain('scale(5)')
    })

    it('clamps scale at minimum (1x)', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')
      // Scroll down many times below minimum
      for (let i = 0; i < 10; i++) {
        fireEvent.wheel(container, { deltaY: 100 })
      }

      const img = screen.getByRole('img', { name: 'Album artwork' })
      expect((img as HTMLImageElement).style.transform).toContain('scale(1)')
    })

    it('double-click zooms in when at fit (1x)', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')
      fireEvent.doubleClick(container)

      const img = screen.getByRole('img', { name: 'Album artwork' })
      expect((img as HTMLImageElement).style.transform).toContain('scale(2)')
    })

    it('double-click zooms back to fit when already zoomed', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')
      fireEvent.doubleClick(container)
      fireEvent.doubleClick(container)

      const img = screen.getByRole('img', { name: 'Album artwork' })
      expect((img as HTMLImageElement).style.transform).toContain('scale(1)')
    })
  })

  describe('pan', () => {
    it('updates translate when dragged while zoomed', () => {
      openViewer()
      render(<ArtworkViewer />)

      const container = screen.getByTestId('artwork-image-container')

      // Zoom in first
      fireEvent.wheel(container, { deltaY: -100 })

      // Drag from (100, 100) to (150, 120)
      fireEvent.mouseDown(container, { clientX: 100, clientY: 100 })
      fireEvent.mouseMove(container, { clientX: 150, clientY: 120 })
      fireEvent.mouseUp(container)

      const img = screen.getByRole('img', { name: 'Album artwork' })
      const transform = (img as HTMLImageElement).style.transform
      // translate should reflect the drag delta
      expect(transform).toMatch(/translate\(/)
    })
  })

  describe('store', () => {
    it('open sets url and isOpen', () => {
      const url = 'http://example.com/art.jpg'
      useArtworkViewer.getState().open(url)
      const state = useArtworkViewer.getState()
      expect(state.isOpen).toBe(true)
      expect(state.url).toBe(url)
    })

    it('close resets isOpen', () => {
      useArtworkViewer.getState().open('http://example.com/art.jpg')
      useArtworkViewer.getState().close()
      expect(useArtworkViewer.getState().isOpen).toBe(false)
    })
  })
})
