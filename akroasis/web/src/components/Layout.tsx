import type { ReactNode } from 'react'
import { Navigation } from './Navigation'

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-bronze-50">
      <Navigation />
      <main>{children}</main>
    </div>
  )
}
