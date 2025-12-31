// Authentication state store
import { create } from 'zustand'
import { apiClient } from '../api/client'

interface AuthState {
  isAuthenticated: boolean
  serverUrl: string
  login: (serverUrl: string, username: string, password: string) => Promise<void>
  logout: () => void
  setServerUrl: (url: string) => void
}

export const useAuthStore = create<AuthState>((set) => ({
  isAuthenticated: !!localStorage.getItem('apiKey'),
  serverUrl: localStorage.getItem('serverUrl') || '',

  login: async (serverUrl: string, username: string, password: string) => {
    apiClient.setServerUrl(serverUrl)
    const response = await apiClient.login(username, password)
    apiClient.setApiKey(response.token)
    set({ isAuthenticated: true, serverUrl })
  },

  logout: () => {
    apiClient.clearAuth()
    set({ isAuthenticated: false })
  },

  setServerUrl: (url: string) => {
    apiClient.setServerUrl(url)
    set({ serverUrl: url })
  },
}))
