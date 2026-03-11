// metaxisStore — crossfade store tests
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useMetaxisStore } from './metaxisStore'

const storage = new Map<string, string>()
vi.stubGlobal('localStorage', {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => storage.set(key, val),
  removeItem: (key: string) => storage.delete(key),
  clear: () => storage.clear(),
})

function resetStore() {
  storage.clear()
  useMetaxisStore.setState({
    mode: 'off',
    duration: 3,
    curve: 'equalPower',
    respectAlbumTransitions: true,
  })
}

describe('metaxisStore', () => {
  beforeEach(resetStore)

  it('defaults to off mode with 3s equalPower', () => {
    const state = useMetaxisStore.getState()
    expect(state.mode).toBe('off')
    expect(state.duration).toBe(3)
    expect(state.curve).toBe('equalPower')
    expect(state.respectAlbumTransitions).toBe(true)
  })

  it('changes mode and persists', () => {
    useMetaxisStore.getState().setMode('simple')
    expect(useMetaxisStore.getState().mode).toBe('simple')
    const stored = JSON.parse(storage.get('akroasis_metaxis')!)
    expect(stored.mode).toBe('simple')
  })

  it('clamps duration to 0-12', () => {
    useMetaxisStore.getState().setDuration(-1)
    expect(useMetaxisStore.getState().duration).toBe(0)

    useMetaxisStore.getState().setDuration(20)
    expect(useMetaxisStore.getState().duration).toBe(12)

    useMetaxisStore.getState().setDuration(5)
    expect(useMetaxisStore.getState().duration).toBe(5)
  })

  it('changes curve', () => {
    useMetaxisStore.getState().setCurve('sCurve')
    expect(useMetaxisStore.getState().curve).toBe('sCurve')
  })

  it('toggles respect album transitions', () => {
    useMetaxisStore.getState().setRespectAlbumTransitions(false)
    expect(useMetaxisStore.getState().respectAlbumTransitions).toBe(false)
  })

  describe('shouldCrossfade', () => {
    it('returns false when mode is off', () => {
      expect(useMetaxisStore.getState().shouldCrossfade('Album A', 'Album B')).toBe(false)
    })

    it('returns true for different albums when enabled', () => {
      useMetaxisStore.getState().setMode('simple')
      expect(useMetaxisStore.getState().shouldCrossfade('Album A', 'Album B')).toBe(true)
    })

    it('returns false for same album when respect album transitions on', () => {
      useMetaxisStore.getState().setMode('simple')
      expect(useMetaxisStore.getState().shouldCrossfade('Album A', 'Album A')).toBe(false)
    })

    it('returns true for same album when respect album transitions off', () => {
      useMetaxisStore.getState().setMode('simple')
      useMetaxisStore.getState().setRespectAlbumTransitions(false)
      expect(useMetaxisStore.getState().shouldCrossfade('Album A', 'Album A')).toBe(true)
    })

    it('returns true when album info is undefined', () => {
      useMetaxisStore.getState().setMode('simple')
      expect(useMetaxisStore.getState().shouldCrossfade(undefined, undefined)).toBe(true)
      expect(useMetaxisStore.getState().shouldCrossfade('Album A', undefined)).toBe(true)
    })
  })
})
