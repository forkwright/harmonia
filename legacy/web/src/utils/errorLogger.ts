// Error logger — captures errors in IndexedDB ring buffer and flushes to backend
//
// Three persistence layers:
//   1. In-memory buffer (instant, lost on refresh)
//   2. IndexedDB ring (survives refresh, max 500 entries)
//   3. Backend POST /api/v3/clientlog (survives everything, max 5000 server-side)

export type ErrorEntry = {
  timestamp: string
  level: 'error' | 'warn' | 'info'
  source: string
  message: string
  detail?: string
  url?: string
  stack?: string
}

// ─── In-memory buffer ───────────────────────────────────────────

const memoryBuffer: ErrorEntry[] = []
const MAX_MEMORY = 200

// ─── IndexedDB ring buffer ──────────────────────────────────────

const DB_NAME = 'akroasis-logs'
const DB_VERSION = 1
const STORE_NAME = 'entries'
const MAX_IDB_ENTRIES = 500

let dbPromise: Promise<IDBDatabase> | null = null

function openDB(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise
  dbPromise = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION)
    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        const store = db.createObjectStore(STORE_NAME, { keyPath: 'id', autoIncrement: true })
        store.createIndex('level', 'level', { unique: false })
        store.createIndex('timestamp', 'timestamp', { unique: false })
      }
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => {
      dbPromise = null
      reject(req.error)
    }
  })
  return dbPromise
}

async function idbPut(entry: ErrorEntry): Promise<void> {
  try {
    const db = await openDB()
    const tx = db.transaction(STORE_NAME, 'readwrite')
    const store = tx.objectStore(STORE_NAME)
    store.add(entry)

    // Prune oldest if over limit
    const countReq = store.count()
    countReq.onsuccess = () => {
      if (countReq.result > MAX_IDB_ENTRIES) {
        const cursor = store.openCursor()
        let toDelete = countReq.result - MAX_IDB_ENTRIES
        cursor.onsuccess = () => {
          if (cursor.result && toDelete > 0) {
            cursor.result.delete()
            toDelete--
            cursor.result.continue()
          }
        }
      }
    }
  } catch {
    // IndexedDB unavailable (private browsing, etc) — memory-only
  }
}

/** Read all entries from IndexedDB, newest first */
export async function idbGetAll(): Promise<(ErrorEntry & { id: number })[]> {
  try {
    const db = await openDB()
    return new Promise((resolve) => {
      const tx = db.transaction(STORE_NAME, 'readonly')
      const store = tx.objectStore(STORE_NAME)
      const req = store.getAll()
      req.onsuccess = () => {
        const results = req.result as (ErrorEntry & { id: number })[]
        results.reverse() // newest first
        resolve(results)
      }
      req.onerror = () => resolve([])
    })
  } catch {
    return []
  }
}

/** Clear all entries from IndexedDB */
export async function idbClear(): Promise<void> {
  try {
    const db = await openDB()
    const tx = db.transaction(STORE_NAME, 'readwrite')
    tx.objectStore(STORE_NAME).clear()
  } catch {
    // ignore
  }
}

// ─── Server flush ───────────────────────────────────────────────

const LOG_ENDPOINT = '/api/v3/clientlog'
const serverQueue: ErrorEntry[] = []
let flushTimer: ReturnType<typeof setTimeout> | null = null
let flushInFlight = false

function scheduleFlush() {
  if (flushTimer || flushInFlight) return
  flushTimer = setTimeout(() => {
    flushTimer = null
    void flushToServer()
  }, 2000) // batch within 2s
}

async function flushToServer(): Promise<void> {
  if (serverQueue.length === 0 || flushInFlight) return
  flushInFlight = true

  const batch = serverQueue.splice(0, 50) // max 50 per flush

  try {
    const token = localStorage.getItem('accessToken')
    if (!token) {
      // Can't auth — put them back
      serverQueue.unshift(...batch)
      return
    }

    const resp = await fetch(LOG_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(batch),
    })

    if (!resp.ok && resp.status === 401) {
      // Token expired — put them back, they'll flush after re-login
      serverQueue.unshift(...batch)
    }
    // On other failures, entries are dropped (they're still in IndexedDB)
  } catch {
    // Network failure — put them back for retry
    serverQueue.unshift(...batch)
  } finally {
    flushInFlight = false
    // If there are more queued, schedule another flush
    if (serverQueue.length > 0) {
      scheduleFlush()
    }
  }
}

// ─── Public API ─────────────────────────────────────────────────

function log(level: ErrorEntry['level'], source: string, message: string, detail?: unknown) {
  const entry: ErrorEntry = {
    timestamp: new Date().toISOString(),
    level,
    source,
    message,
    detail: detail instanceof Error ? detail.message : detail ? String(detail) : undefined,
    url: typeof window !== 'undefined' ? window.location.href : undefined,
    stack: detail instanceof Error ? detail.stack : undefined,
  }

  // 1. Console
  if (level === 'error') console.error(`[${source}]`, message, detail)
  else if (level === 'warn') console.warn(`[${source}]`, message, detail)

  // 2. Memory ring
  memoryBuffer.push(entry)
  if (memoryBuffer.length > MAX_MEMORY) memoryBuffer.shift()

  // 3. IndexedDB (async, fire-and-forget)
  void idbPut(entry)

  // 4. Server queue
  serverQueue.push(entry)
  scheduleFlush()
}

export function logError(source: string, message: string, detail?: unknown) {
  log('error', source, message, detail)
}

export function logWarn(source: string, message: string, detail?: unknown) {
  log('warn', source, message, detail)
}

export function logInfo(source: string, message: string, detail?: unknown) {
  log('info', source, message, detail)
}

/** Get in-memory entries (newest first, no async needed) */
export function getMemoryLogs(): readonly ErrorEntry[] {
  return [...memoryBuffer].reverse()
}

// ─── Global error handlers ──────────────────────────────────────

export function installGlobalHandlers() {
  window.addEventListener('error', (event) => {
    logError('global', event.message, event.error)
  })

  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason
    const message = reason instanceof Error ? reason.message : String(reason)
    logError('unhandled-promise', message, reason)
  })

  // Flush remaining entries before page unload
  window.addEventListener('beforeunload', () => {
    if (serverQueue.length > 0) {
      const token = localStorage.getItem('accessToken')
      if (token) {
        // sendBeacon doesn't support custom headers, so use keepalive fetch
        const batch = serverQueue.splice(0, 50)
        void fetch(LOG_ENDPOINT, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`,
          },
          body: JSON.stringify(batch),
          keepalive: true,
        }).catch(() => {})
      }
    }
  })
}
