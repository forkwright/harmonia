import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { scrobbleQueue } from './services/scrobbleQueue'
import { installGlobalHandlers } from './utils/errorLogger'
import { isTauri } from './utils/platform'

installGlobalHandlers()
scrobbleQueue.init()

async function enableMocking() {
  // Disable MSW when talking to a real backend
  // Set VITE_USE_MOCKS=true to re-enable for offline dev
  if (!import.meta.env.VITE_USE_MOCKS) {
    return
  }

  if (import.meta.env.MODE !== 'development') {
    return
  }

  // MSW service worker cannot run inside Tauri's webview
  if (isTauri()) {
    return
  }

  const { worker } = await import('./mocks/browser')

  return worker.start({
    onUnhandledRequest: 'bypass',
  })
}

enableMocking().then(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>
  )
})
