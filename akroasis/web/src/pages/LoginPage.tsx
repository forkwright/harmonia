import { useState } from 'react'
import type { FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'
import { Button } from '../components/Button'
import { Input } from '../components/Input'

export function LoginPage() {
  const savedUrl = localStorage.getItem('serverUrl') || ''
  const [useRemote, setUseRemote] = useState(savedUrl !== '')
  const [serverUrl, setServerUrl] = useState(savedUrl)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const login = useAuthStore((state) => state.login)
  const navigate = useNavigate()

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    setError('')
    setLoading(true)

    try {
      const url = useRemote ? serverUrl : ''
      await login(url, username, password)
      navigate('/player')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div
      className="min-h-screen flex items-center justify-center p-4"
      style={{ backgroundColor: 'rgb(var(--surface-base))' }}
    >
      <div className="w-full max-w-sm">
        {/* Brand */}
        <div className="text-center mb-10">
          <h1
            className="text-5xl font-serif font-semibold tracking-tight mb-1"
            style={{ color: 'rgb(var(--text-primary))' }}
          >
            Akroasis
          </h1>
          <p
            className="text-sm tracking-widest"
            style={{ color: 'rgb(var(--text-muted))' }}
          >
            Ἀκρόασις — a hearing
          </p>
        </div>

        {/* Login card */}
        <div
          className="rounded-lg p-6"
          style={{
            backgroundColor: 'rgb(var(--surface-raised))',
            border: '1px solid rgb(var(--border-default))',
          }}
        >
          <form onSubmit={handleSubmit} className="space-y-4">
            <Input
              type="text"
              label="Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              disabled={loading}
            />

            <Input
              type="password"
              label="Password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              disabled={loading}
            />

            {useRemote && (
              <Input
                type="url"
                label="Server URL"
                placeholder="https://mouseion.example.com"
                value={serverUrl}
                onChange={(e) => setServerUrl(e.target.value)}
                required
                disabled={loading}
              />
            )}

            {error && (
              <div
                className="p-3 rounded-lg text-sm"
                style={{
                  backgroundColor: 'rgb(var(--error-bg))',
                  border: '1px solid rgb(var(--error-border))',
                  color: 'rgb(var(--error-text))',
                }}
              >
                {error}
              </div>
            )}

            <Button
              type="submit"
              variant="primary"
              className="w-full mt-2"
              disabled={loading}
            >
              {loading ? 'Connecting...' : 'Sign in'}
            </Button>
          </form>

          <button
            type="button"
            onClick={() => setUseRemote(!useRemote)}
            className="w-full text-center text-xs mt-4 transition-colors"
            style={{ color: 'rgb(var(--text-muted))' }}
          >
            {useRemote ? 'Use local server' : 'Connect to a remote server'}
          </button>
        </div>
      </div>
    </div>
  )
}
