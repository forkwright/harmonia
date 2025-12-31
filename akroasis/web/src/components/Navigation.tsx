import { useLocation, useNavigate } from 'react-router-dom'
import { Button } from './Button'
import { useAuthStore } from '../stores/authStore'

export function Navigation() {
  const location = useLocation()
  const navigate = useNavigate()
  const logout = useAuthStore((state) => state.logout)

  const isLibrary = location.pathname === '/library'
  const isPlayer = location.pathname === '/player'

  function handleLogout() {
    logout()
    navigate('/login')
  }

  return (
    <nav className="bg-bronze-900 text-bronze-50 shadow-lg">
      <div className="max-w-7xl mx-auto px-4">
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold">Akroasis</h1>
            <span className="text-bronze-400 text-sm">Ἀκρόασις</span>
          </div>

          <div className="flex items-center gap-4">
            <Button
              variant={isLibrary ? 'primary' : 'secondary'}
              onClick={() => navigate('/library')}
              className="min-w-24"
            >
              Library
            </Button>
            <Button
              variant={isPlayer ? 'primary' : 'secondary'}
              onClick={() => navigate('/player')}
              className="min-w-24"
            >
              Player
            </Button>
            <Button variant="secondary" onClick={handleLogout}>
              Logout
            </Button>
          </div>
        </div>
      </div>
    </nav>
  )
}
