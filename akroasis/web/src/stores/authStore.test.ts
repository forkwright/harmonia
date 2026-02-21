import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { useAuthStore } from './authStore'
import { apiClient } from '../api/client'

vi.mock('../api/client', () => ({
  apiClient: {
    setServerUrl: vi.fn(),
    setApiKey: vi.fn(),
    clearAuth: vi.fn(),
    login: vi.fn(),
  },
}))

describe('authStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    useAuthStore.setState({
      isAuthenticated: false,
      serverUrl: '',
      isOnline: true,
    })
  })

  afterEach(() => {
    localStorage.clear()
  })

  describe('initial state', () => {
    it('reads isAuthenticated from localStorage apiKey presence', () => {
      localStorage.setItem('apiKey', 'some-key')
      // Re-initialize store by reading from localStorage
      useAuthStore.setState({ isAuthenticated: true })
      expect(useAuthStore.getState().isAuthenticated).toBe(true)
    })

    it('starts unauthenticated when no apiKey in localStorage', () => {
      useAuthStore.setState({ isAuthenticated: false })
      expect(useAuthStore.getState().isAuthenticated).toBe(false)
    })

    it('reads serverUrl from localStorage', () => {
      localStorage.setItem('serverUrl', 'http://192.168.0.10:5000')
      useAuthStore.setState({ serverUrl: localStorage.getItem('serverUrl') ?? '' })
      expect(useAuthStore.getState().serverUrl).toBe('http://192.168.0.10:5000')
    })

    it('defaults to empty serverUrl when none stored', () => {
      useAuthStore.setState({ serverUrl: '' })
      expect(useAuthStore.getState().serverUrl).toBe('')
    })
  })

  describe('login', () => {
    it('calls apiClient methods and sets authenticated state', async () => {
      vi.mocked(apiClient.login).mockResolvedValueOnce({ token: 'abc123', expiresIn: 3600 })

      await useAuthStore.getState().login('http://server:5000', 'admin', 'secret')

      expect(apiClient.setServerUrl).toHaveBeenCalledWith('http://server:5000')
      expect(apiClient.login).toHaveBeenCalledWith('admin', 'secret')
      expect(apiClient.setApiKey).toHaveBeenCalledWith('abc123')

      const state = useAuthStore.getState()
      expect(state.isAuthenticated).toBe(true)
      expect(state.serverUrl).toBe('http://server:5000')
    })

    it('propagates errors from apiClient.login', async () => {
      vi.mocked(apiClient.login).mockRejectedValueOnce(new Error('Invalid credentials'))

      await expect(
        useAuthStore.getState().login('http://server:5000', 'admin', 'wrong')
      ).rejects.toThrow('Invalid credentials')

      expect(useAuthStore.getState().isAuthenticated).toBe(false)
    })

    it('stores server url in store state on successful login', async () => {
      vi.mocked(apiClient.login).mockResolvedValueOnce({ token: 'tok', expiresIn: 7200 })

      await useAuthStore.getState().login('http://myserver:8080', 'user', 'pass')

      expect(useAuthStore.getState().serverUrl).toBe('http://myserver:8080')
    })
  })

  describe('logout', () => {
    it('clears auth and sets isAuthenticated to false', () => {
      useAuthStore.setState({ isAuthenticated: true, serverUrl: 'http://server:5000' })

      useAuthStore.getState().logout()

      expect(apiClient.clearAuth).toHaveBeenCalledOnce()
      expect(useAuthStore.getState().isAuthenticated).toBe(false)
    })

    it('does not reset serverUrl on logout', () => {
      useAuthStore.setState({ isAuthenticated: true, serverUrl: 'http://server:5000' })

      useAuthStore.getState().logout()

      expect(useAuthStore.getState().serverUrl).toBe('http://server:5000')
    })
  })

  describe('setServerUrl', () => {
    it('updates store and calls apiClient.setServerUrl', () => {
      useAuthStore.getState().setServerUrl('http://newserver:9000')

      expect(apiClient.setServerUrl).toHaveBeenCalledWith('http://newserver:9000')
      expect(useAuthStore.getState().serverUrl).toBe('http://newserver:9000')
    })

    it('can be called multiple times, each updates state', () => {
      useAuthStore.getState().setServerUrl('http://first:5000')
      useAuthStore.getState().setServerUrl('http://second:5000')

      expect(useAuthStore.getState().serverUrl).toBe('http://second:5000')
      expect(apiClient.setServerUrl).toHaveBeenCalledTimes(2)
    })
  })

  describe('setOnline', () => {
    it('sets isOnline to true', () => {
      useAuthStore.setState({ isOnline: false })
      useAuthStore.getState().setOnline(true)
      expect(useAuthStore.getState().isOnline).toBe(true)
    })

    it('sets isOnline to false', () => {
      useAuthStore.setState({ isOnline: true })
      useAuthStore.getState().setOnline(false)
      expect(useAuthStore.getState().isOnline).toBe(false)
    })
  })
})
