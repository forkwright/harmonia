// Full-screen artwork lightbox with zoom and pan
import { useEffect, useRef, useCallback, useState } from 'react'
import { createPortal } from 'react-dom'
import { useArtworkViewer } from '../stores/artworkViewerStore'

const MIN_SCALE = 1
const MAX_SCALE = 5
const ZOOM_STEP = 0.25
const DOUBLE_TAP_MS = 300

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

interface ArtworkContentProps {
  url: string
  close: () => void
}

function ArtworkContent({ url, close }: ArtworkContentProps) {
  const [scale, setScale] = useState(1)
  const [translate, setTranslate] = useState({ x: 0, y: 0 })
  const [dragging, setDragging] = useState(false)

  const isDragging = useRef(false)
  const dragStart = useRef({ x: 0, y: 0 })
  const translateAtDragStart = useRef({ x: 0, y: 0 })
  const lastTapTime = useRef(0)
  const pinchStartDistance = useRef<number | null>(null)
  const pinchStartScale = useRef(1)

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [close])

  useEffect(() => {
    const prev = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => { document.body.style.overflow = prev }
  }, [])

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP
    setScale((prev) => {
      const next = clamp(prev + delta, MIN_SCALE, MAX_SCALE)
      if (next === MIN_SCALE) setTranslate({ x: 0, y: 0 })
      return next
    })
  }, [])

  const handleDoubleClick = useCallback(() => {
    setScale((prev) => {
      if (prev > MIN_SCALE) {
        setTranslate({ x: 0, y: 0 })
        return MIN_SCALE
      }
      return 2
    })
  }, [])

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    isDragging.current = true
    setDragging(true)
    dragStart.current = { x: e.clientX, y: e.clientY }
    translateAtDragStart.current = translate
  }, [translate])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!isDragging.current) return
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    setTranslate({
      x: translateAtDragStart.current.x + dx,
      y: translateAtDragStart.current.y + dy,
    })
  }, [])

  const stopDrag = useCallback(() => {
    isDragging.current = false
    setDragging(false)
  }, [])

  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    if (e.touches.length === 2) {
      const dx = e.touches[1].clientX - e.touches[0].clientX
      const dy = e.touches[1].clientY - e.touches[0].clientY
      pinchStartDistance.current = Math.hypot(dx, dy)
      pinchStartScale.current = scale
    } else if (e.touches.length === 1) {
      const now = Date.now()
      if (now - lastTapTime.current < DOUBLE_TAP_MS) {
        handleDoubleClick()
      }
      lastTapTime.current = now
      isDragging.current = true
      setDragging(true)
      dragStart.current = { x: e.touches[0].clientX, y: e.touches[0].clientY }
      translateAtDragStart.current = translate
    }
  }, [scale, translate, handleDoubleClick])

  const handleTouchMove = useCallback((e: React.TouchEvent) => {
    e.preventDefault()
    if (e.touches.length === 2 && pinchStartDistance.current !== null) {
      const dx = e.touches[1].clientX - e.touches[0].clientX
      const dy = e.touches[1].clientY - e.touches[0].clientY
      const dist = Math.hypot(dx, dy)
      const ratio = dist / pinchStartDistance.current
      const next = clamp(pinchStartScale.current * ratio, MIN_SCALE, MAX_SCALE)
      setScale(next)
      if (next === MIN_SCALE) setTranslate({ x: 0, y: 0 })
    } else if (e.touches.length === 1 && isDragging.current) {
      const dx = e.touches[0].clientX - dragStart.current.x
      const dy = e.touches[0].clientY - dragStart.current.y
      setTranslate({
        x: translateAtDragStart.current.x + dx,
        y: translateAtDragStart.current.y + dy,
      })
    }
  }, [])

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Artwork viewer"
      data-testid="artwork-viewer"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        animation: 'artworkFadeIn 150ms ease',
      }}
    >
      {/* Backdrop */}
      <div
        data-testid="artwork-backdrop"
        onClick={close}
        style={{
          position: 'absolute',
          inset: 0,
          backgroundColor: 'rgba(0, 0, 0, 0.92)',
        }}
      />

      {/* Close button */}
      <button
        onClick={close}
        aria-label="Close artwork viewer"
        style={{
          position: 'absolute',
          top: '1rem',
          right: '1rem',
          zIndex: 1,
          background: 'rgba(0,0,0,0.5)',
          border: '1px solid rgba(255,255,255,0.2)',
          borderRadius: '50%',
          width: '2.5rem',
          height: '2.5rem',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          color: 'rgba(255,255,255,0.8)',
        }}
      >
        <svg width="16" height="16" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      {/* Image container */}
      <div
        data-testid="artwork-image-container"
        style={{
          position: 'relative',
          zIndex: 1,
          maxWidth: '90vw',
          maxHeight: '90vh',
          cursor: scale > 1 ? 'grab' : 'zoom-in',
          userSelect: 'none',
          touchAction: 'none',
        }}
        onWheel={handleWheel}
        onDoubleClick={handleDoubleClick}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={stopDrag}
        onMouseLeave={stopDrag}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={stopDrag}
      >
        <img
          src={url}
          alt="Album artwork"
          draggable={false}
          style={{
            display: 'block',
            maxWidth: '90vw',
            maxHeight: '90vh',
            objectFit: 'contain',
            borderRadius: '0.5rem',
            transform: `scale(${scale}) translate(${translate.x / scale}px, ${translate.y / scale}px)`,
            transformOrigin: 'center center',
            transition: dragging ? 'none' : 'transform 150ms ease',
          }}
        />
      </div>

      <style>{`
        @keyframes artworkFadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
      `}</style>
    </div>
  )
}

export function ArtworkViewer() {
  const { isOpen, url, close } = useArtworkViewer()

  if (!isOpen) return null

  return createPortal(
    <ArtworkContent key={url} url={url} close={close} />,
    document.body
  )
}
