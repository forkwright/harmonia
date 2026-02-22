// Error logger — captures all errors and sends them to a local logging endpoint
// In dev mode, also writes to console with full context

type ErrorEntry = {
  timestamp: string
  level: 'error' | 'warn' | 'info'
  source: string
  message: string
  detail?: string
  url?: string
  stack?: string
}

const LOG_ENDPOINT = '/api/v3/__dev/log' // intercepted by Vite proxy → file
const errorBuffer: ErrorEntry[] = []
let flushTimer: ReturnType<typeof setTimeout> | null = null

function createEntry(level: ErrorEntry['level'], source: string, message: string, detail?: unknown): ErrorEntry {
  return {
    timestamp: new Date().toISOString(),
    level,
    source,
    message,
    detail: detail instanceof Error ? detail.message : detail ? String(detail) : undefined,
    url: typeof window !== 'undefined' ? window.location.href : undefined,
    stack: detail instanceof Error ? detail.stack : undefined,
  }
}

function flushToServer() {
  if (errorBuffer.length === 0) return
  const batch = errorBuffer.splice(0)
  
  // Fire and forget — don't let logging errors cascade
  fetch(LOG_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(batch),
  }).catch(() => {
    // If server logging fails, at least keep console output
  })
}

function scheduleFlush() {
  if (flushTimer) return
  flushTimer = setTimeout(() => {
    flushTimer = null
    flushToServer()
  }, 500) // batch within 500ms
}

export function logError(source: string, message: string, detail?: unknown) {
  const entry = createEntry('error', source, message, detail)
  console.error(`[${source}]`, message, detail)
  errorBuffer.push(entry)
  scheduleFlush()
}

export function logWarn(source: string, message: string, detail?: unknown) {
  const entry = createEntry('warn', source, message, detail)
  console.warn(`[${source}]`, message, detail)
  errorBuffer.push(entry)
  scheduleFlush()
}

export function logInfo(source: string, message: string, detail?: unknown) {
  const entry = createEntry('info', source, message, detail)
  errorBuffer.push(entry)
  scheduleFlush()
}

// Global error handlers
export function installGlobalHandlers() {
  window.addEventListener('error', (event) => {
    logError('global', event.message, event.error)
  })

  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason
    const message = reason instanceof Error ? reason.message : String(reason)
    logError('unhandled-promise', message, reason)
  })
}
