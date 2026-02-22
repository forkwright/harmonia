import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import type { ReactNode } from 'react'
import { useAuthStore } from './stores/authStore'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useMediaSession } from './hooks/useMediaSession'
import { Layout } from './components/Layout'
import { OfflineIndicator } from './components/OfflineIndicator'
import { ArtworkViewer } from './components/ArtworkViewer'
import { LoginPage } from './pages/LoginPage'
import { PlayerPage } from './pages/PlayerPage'
import { LibraryPage } from './pages/LibraryPage'
import { QueuePage } from './pages/QueuePage'
import { SettingsPage } from './pages/SettingsPage'
import { AudiobooksPage } from './pages/AudiobooksPage'
import { AudiobookPlayerPage } from './pages/AudiobookPlayerPage'
import { DiscoveryPage } from './pages/DiscoveryPage'
import { PodcastsPage } from './pages/PodcastsPage'

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
          path="/audiobooks"
          element={
            <PrivateRoute>
              <AudiobooksPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/audiobooks/play/:id"
          element={
            <PrivateRoute>
              <AudiobookPlayerPage />
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
        <Route
          path="/settings"
          element={
            <PrivateRoute>
              <SettingsPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/discover"
          element={
            <PrivateRoute>
              <DiscoveryPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/podcasts"
          element={
            <PrivateRoute>
              <PodcastsPage />
            </PrivateRoute>
          }
        />
        <Route path="/" element={<Navigate to="/audiobooks" replace />} />
    </Routes>
  )
}

export default function App() {
  return (
    <BrowserRouter>
      <OfflineIndicator />
      <AppContent />
      <ArtworkViewer />
    </BrowserRouter>
  )
}
