// Diagnostics page — view client and server error logs
import { useState, useEffect, useCallback } from 'react'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { idbGetAll, idbClear, type ErrorEntry } from '../utils/errorLogger'

type ServerLogEntry = ErrorEntry & { id: number; userAgent?: string; receivedAt?: string }
type Tab = 'local' | 'server'
type LevelFilter = 'all' | 'error' | 'warn' | 'info'

const levelColors: Record<string, string> = {
  error: 'rgb(220, 38, 38)',
  warn: 'rgb(217, 119, 6)',
  info: 'rgb(59, 130, 246)',
}

function RelativeTime({ iso }: { iso: string }) {
  const d = new Date(iso)
  const now = Date.now()
  const diffS = Math.floor((now - d.getTime()) / 1000)
  if (diffS < 60) return <span>{diffS}s ago</span>
  if (diffS < 3600) return <span>{Math.floor(diffS / 60)}m ago</span>
  if (diffS < 86400) return <span>{Math.floor(diffS / 3600)}h ago</span>
  return <span>{d.toLocaleDateString()}</span>
}

function LogRow({ entry, expanded, onToggle }: {
  entry: ErrorEntry & { id?: number }
  expanded: boolean
  onToggle: () => void
}) {
  return (
    <div
      style={{ borderBottom: '1px solid rgb(var(--border-subtle))' }}
      className="text-sm"
    >
      <div
        className="flex items-start gap-3 py-2 px-3 cursor-pointer hover:opacity-80"
        onClick={onToggle}
      >
        <span
          className="font-mono text-xs font-bold uppercase shrink-0 mt-0.5"
          style={{ color: levelColors[entry.level] || 'inherit', minWidth: '2.5rem' }}
        >
          {entry.level}
        </span>
        <span
          className="font-mono text-xs shrink-0 mt-0.5"
          style={{ color: 'rgb(var(--text-secondary))', minWidth: '4.5rem' }}
        >
          {entry.source}
        </span>
        <span className="flex-1 truncate" style={{ color: 'rgb(var(--text-primary))' }}>
          {entry.message}
        </span>
        <span
          className="text-xs shrink-0"
          style={{ color: 'rgb(var(--text-secondary))' }}
        >
          <RelativeTime iso={entry.timestamp} />
        </span>
      </div>

      {expanded && (
        <div
          className="px-3 pb-3 space-y-2 text-xs font-mono"
          style={{ color: 'rgb(var(--text-secondary))' }}
        >
          <div><strong>Time:</strong> {entry.timestamp}</div>
          {entry.url && <div><strong>URL:</strong> {entry.url}</div>}
          {entry.detail && <div><strong>Detail:</strong> {entry.detail}</div>}
          {entry.stack && (
            <pre
              className="overflow-x-auto p-2 rounded text-xs whitespace-pre-wrap"
              style={{ backgroundColor: 'rgb(var(--surface-base))' }}
            >
              {entry.stack}
            </pre>
          )}
        </div>
      )}
    </div>
  )
}

export function DiagnosticsPage() {
  const [tab, setTab] = useState<Tab>('server')
  const [level, setLevel] = useState<LevelFilter>('all')
  const [localEntries, setLocalEntries] = useState<(ErrorEntry & { id: number })[]>([])
  const [serverEntries, setServerEntries] = useState<ServerLogEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [copyMsg, setCopyMsg] = useState('')

  const loadLocal = useCallback(async () => {
    const entries = await idbGetAll()
    setLocalEntries(entries)
  }, [])

  const loadServer = useCallback(async () => {
    setLoading(true)
    try {
      const params = new URLSearchParams({ limit: '200' })
      if (level !== 'all') params.set('level', level)
      const resp = await fetch(`/api/v3/clientlog?${params}`, {
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('accessToken') || ''}`,
        },
      })
      if (resp.ok) {
        setServerEntries(await resp.json())
      }
    } catch {
      // silently fail
    } finally {
      setLoading(false)
    }
  }, [level])

  useEffect(() => {
    if (tab === 'local') void loadLocal()
    else void loadServer()
  }, [tab, loadLocal, loadServer])

  const entries = tab === 'local'
    ? (level === 'all' ? localEntries : localEntries.filter(e => e.level === level))
    : serverEntries

  const handleClear = async () => {
    if (tab === 'local') {
      await idbClear()
      setLocalEntries([])
    } else {
      const token = localStorage.getItem('accessToken') || ''
      await fetch('/api/v3/clientlog', {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` },
      })
      setServerEntries([])
    }
  }

  const handleCopy = async () => {
    const text = entries.map(e =>
      `[${e.timestamp}] ${e.level.toUpperCase()} [${e.source}] ${e.message}${e.detail ? `\n  Detail: ${e.detail}` : ''}${e.stack ? `\n  Stack: ${e.stack}` : ''}`
    ).join('\n\n')

    await navigator.clipboard.writeText(text)
    setCopyMsg('Copied!')
    setTimeout(() => setCopyMsg(''), 2000)
  }

  return (
    <div className="container mx-auto p-6 max-w-5xl">
      <h1
        className="text-3xl font-serif font-semibold mb-6"
        style={{ color: 'rgb(var(--text-primary))' }}
      >
        Diagnostics
      </h1>

      {/* Tab + filter bar */}
      <div className="flex flex-wrap items-center gap-3 mb-4">
        <div className="flex gap-1">
          {(['server', 'local'] as Tab[]).map(t => (
            <Button
              key={t}
              size="sm"
              variant={tab === t ? 'primary' : 'secondary'}
              onClick={() => setTab(t)}
            >
              {t === 'server' ? 'Server Logs' : 'Local (IndexedDB)'}
            </Button>
          ))}
        </div>

        <div className="flex gap-1">
          {(['all', 'error', 'warn', 'info'] as LevelFilter[]).map(l => (
            <Button
              key={l}
              size="sm"
              variant={level === l ? 'primary' : 'ghost'}
              onClick={() => setLevel(l)}
            >
              {l === 'all' ? 'All' : l.charAt(0).toUpperCase() + l.slice(1)}
            </Button>
          ))}
        </div>

        <div className="flex-1" />

        <Button size="sm" variant="ghost" onClick={handleCopy}>
          {copyMsg || 'Copy All'}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => tab === 'local' ? loadLocal() : loadServer()}
        >
          Refresh
        </Button>
        <Button size="sm" variant="ghost" onClick={handleClear}>
          Clear
        </Button>
      </div>

      {/* Entry count */}
      <div className="text-xs mb-2" style={{ color: 'rgb(var(--text-secondary))' }}>
        {loading ? 'Loading...' : `${entries.length} entries`}
        {tab === 'server' && ' — stored on Mouseion in logs.db'}
        {tab === 'local' && ' — stored in browser IndexedDB'}
      </div>

      {/* Log entries */}
      <Card className="p-0 overflow-hidden">
        {entries.length === 0 ? (
          <div
            className="p-8 text-center text-sm"
            style={{ color: 'rgb(var(--text-secondary))' }}
          >
            No log entries{level !== 'all' ? ` at ${level} level` : ''}
          </div>
        ) : (
          <div className="max-h-[70vh] overflow-y-auto">
            {entries.map((entry, i) => {
              const key = `${tab}-${'id' in entry ? entry.id : i}`
              return (
                <LogRow
                  key={key}
                  entry={entry}
                  expanded={expandedId === key}
                  onToggle={() => setExpandedId(expandedId === key ? null : key)}
                />
              )
            })}
          </div>
        )}
      </Card>

      {/* Info footer */}
      <div
        className="mt-4 text-xs space-y-1"
        style={{ color: 'rgb(var(--text-secondary))' }}
      >
        <p><strong>Local:</strong> IndexedDB ring buffer, max 500 entries, survives page refresh. Lost on cache clear.</p>
        <p><strong>Server:</strong> Mouseion logs.db, max 5000 entries, auto-pruned. Readable via SSH or this page.</p>
        <p><strong>CLI:</strong> <code style={{ backgroundColor: 'rgb(var(--surface-base))', padding: '2px 4px', borderRadius: '3px' }}>
          sqlite3 ~/docker_configs/mouseion/config/logs.db "SELECT * FROM ClientLog ORDER BY Id DESC LIMIT 20;"
        </code></p>
      </div>
    </div>
  )
}
