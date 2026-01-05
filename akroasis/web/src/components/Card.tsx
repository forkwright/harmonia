import type { HTMLAttributes, ReactNode } from 'react'

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  readonly children: ReactNode
}

export function Card({ children, className = '', ...props }: CardProps) {
  return (
    <div
      className={`bg-bronze-900 border border-bronze-800 rounded-lg p-6 ${className}`}
      {...props}
    >
      {children}
    </div>
  )
}
