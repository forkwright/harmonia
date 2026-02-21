import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { scrobbleQueue } from './services/scrobbleQueue'
import { isTauri } from './utils/platform'

scrobbleQueue.init()

async function enableMocking() {
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
