import { describe, it, expect, vi, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useDebounce } from './useDebounce'

describe('useDebounce', () => {
  afterEach(() => vi.restoreAllMocks())

  it('returns initial value immediately', () => {
    const { result } = renderHook(() => useDebounce('hello', 300))
    expect(result.current).toBe('hello')
  })

  it('updates after delay', () => {
    vi.useFakeTimers()
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 300),
      { initialProps: { value: 'a' } },
    )

    rerender({ value: 'b' })
    expect(result.current).toBe('a')

    act(() => { vi.advanceTimersByTime(300) })
    expect(result.current).toBe('b')

    vi.useRealTimers()
  })

  it('only emits last value on rapid changes', () => {
    vi.useFakeTimers()
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 300),
      { initialProps: { value: 'a' } },
    )

    rerender({ value: 'b' })
    act(() => { vi.advanceTimersByTime(100) })
    rerender({ value: 'c' })
    act(() => { vi.advanceTimersByTime(100) })
    rerender({ value: 'd' })
    act(() => { vi.advanceTimersByTime(300) })

    expect(result.current).toBe('d')

    vi.useRealTimers()
  })
})
