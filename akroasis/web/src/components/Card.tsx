import type { HTMLAttributes, KeyboardEvent, ReactNode } from 'react'

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  readonly children: ReactNode
}

export function Card({ children, className = '', onClick, ...props }: CardProps) {
  const isClickable = !!onClick

  const handleKeyDown = isClickable
    ? (e: KeyboardEvent<HTMLDivElement>) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick?.(e as unknown as React.MouseEvent<HTMLDivElement>) } }
    : undefined

  return (
    <div
      className={`bg-bronze-900 border border-bronze-800 rounded-lg p-6 ${isClickable ? 'cursor-pointer' : ''} ${className}`}
      onClick={onClick}
      onKeyDown={handleKeyDown}
      {...(isClickable && { role: 'button', tabIndex: 0 })}
      {...props}
    >
      {children}
    </div>
  )
}
