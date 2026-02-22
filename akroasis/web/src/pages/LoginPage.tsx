import { useState } from 'react'
import type { FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'
import { Button } from '../components/Button'
import { Input } from '../components/Input'

export function LoginPage() {
  const [serverUrl, setServerUrl] = useState(
    import.meta.env.MODE === 'development' ? 'http://localhost:5000' : ''
  )
  const [username, setUsername] = useState(
    import.meta.env.MODE === 'development' ? 'admin' : ''
  )
  const [password, setPassword] = useState(
    import.meta.env.MODE === 'development' ? 'password' : ''
  )
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const login = useAuthStore((state) => state.login)
  const navigate = useNavigate()

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    setError('')
    setLoading(true)

    try {
      await login(serverUrl, username, password)
      navigate('/player')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-neutral-950">
      <div className="w-full max-w-sm">
        {/* Brand */}
        <div className="text-center mb-10">
          <h1 className="text-5xl font-bold text-bronze-200 tracking-tight mb-1">Akroasis</h1>
          <p className="text-bronze-500 text-sm tracking-widest">Ἀκρόασις — a hearing</p>
        </div>

        {/* Login card */}
        <div className="bg-bronze-900/50 border border-bronze-800/50 rounded-2xl p-6 backdrop-blur-sm">
          <p className="text-bronze-400 text-sm mb-6">Connect to your Mouseion server</p>

          <form onSubmit={handleSubmit} className="space-y-4">
            <Input
              type="url"
              label="Server URL"
              placeholder="https://example.com:5000"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              required
              disabled={loading}
            />

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

            {error && (
              <div className="p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-red-300 text-sm">
                {error}
              </div>
            )}

            <Button
              type="submit"
              variant="primary"
              className="w-full mt-2"
              disabled={loading}
            >
              {loading ? 'Connecting...' : 'Connect'}
            </Button>
          </form>
        </div>

        <p className="text-center text-bronze-700 text-xs mt-6">
          Self-hosted media player — no cloud, no tracking
        </p>
      </div>
    </div>
  )
}
