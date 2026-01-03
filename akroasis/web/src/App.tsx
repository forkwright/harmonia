import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import type { ReactNode } from 'react'
import { useAuthStore } from './stores/authStore'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useMediaSession } from './hooks/useMediaSession'
import { Layout } from './components/Layout'
import { OfflineIndicator } from './components/OfflineIndicator'
import { LoginPage } from './pages/LoginPage'
import { PlayerPage } from './pages/PlayerPage'
import { LibraryPage } from './pages/LibraryPage'
import { QueuePage } from './pages/QueuePage'

function PrivateRoute({ children }: { children: ReactNode }) {
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated)
  return isAuthenticated ? <Layout>{children}</Layout> : <Navigate to="/login" replace />
}

function AppContent() {
  useKeyboardShortcuts()
  useMediaSession()

  return (
    <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/library"
          element={
            <PrivateRoute>
              <LibraryPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/player"
          element={
            <PrivateRoute>
              <PlayerPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/queue"
          element={
            <PrivateRoute>
              <QueuePage />
            </PrivateRoute>
          }
        />
        <Route path="/" element={<Navigate to="/library" replace />} />
    </Routes>
  )
}

export default function App() {
  return (
    <BrowserRouter>
      <OfflineIndicator />
      <AppContent />
    </BrowserRouter>
  )
}
