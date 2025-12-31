import { useState } from 'react'
import type { FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'
import { Button } from '../components/Button'
import { Input } from '../components/Input'
import { Card } from '../components/Card'

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
    <div className="min-h-screen flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold text-bronze-400 mb-2">Akroasis</h1>
          <p className="text-bronze-500">Connect to Mouseion</p>
        </div>

        <Card>
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
              <div className="p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
                {error}
              </div>
            )}

            <Button
              type="submit"
              variant="primary"
              className="w-full"
              disabled={loading}
            >
              {loading ? 'Connecting...' : 'Login'}
            </Button>
          </form>
        </Card>
      </div>
    </div>
  )
}
