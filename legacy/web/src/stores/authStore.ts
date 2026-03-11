// Authentication state store
import { create } from 'zustand'
import { apiClient } from '../api/client'
import type { User } from '../types'
import { loadJson } from '../utils/storage'

interface AuthState {
  isAuthenticated: boolean
  user: User | null
  serverUrl: string
  isOnline: boolean
  login: (serverUrl: string, username: string, password: string) => Promise<void>
  logout: () => void
  setServerUrl: (url: string) => void
  setOnline: (online: boolean) => void
}

export const useAuthStore = create<AuthState>((set) => {
  apiClient.setOnLogout(() => {
    set({ isAuthenticated: false, user: null })
  })

  return {
    isAuthenticated: !!localStorage.getItem('accessToken'),
    user: loadJson<User | null>('user', null),
    serverUrl: localStorage.getItem('serverUrl') || '',
    isOnline: navigator.onLine,

    login: async (serverUrl: string, username: string, password: string) => {
      apiClient.setServerUrl(serverUrl)
      const response = await apiClient.login(username, password)
      apiClient.setTokens(response.accessToken, response.refreshToken)
      localStorage.setItem('user', JSON.stringify(response.user))
      set({ isAuthenticated: true, serverUrl, user: response.user })
    },

    logout: () => {
      apiClient.logout()
      localStorage.removeItem('user')
      set({ isAuthenticated: false, user: null })
    },

    setServerUrl: (url: string) => {
      apiClient.setServerUrl(url)
      set({ serverUrl: url })
    },

    setOnline: (online: boolean) => {
      set({ isOnline: online })
    },
  }
})

if (typeof globalThis.window !== 'undefined') {
  globalThis.addEventListener('online', () => {
    useAuthStore.getState().setOnline(true)
  })
  globalThis.addEventListener('offline', () => {
    useAuthStore.getState().setOnline(false)
  })
}
