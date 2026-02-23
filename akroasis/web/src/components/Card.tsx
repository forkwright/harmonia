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
      className={`rounded-lg p-6 ${isClickable ? 'cursor-pointer' : ''} ${className}`}
      style={{
        backgroundColor: 'rgb(var(--surface-raised))',
        border: '1px solid rgb(var(--border-subtle))',
      }}
      onClick={onClick}
      onKeyDown={handleKeyDown}
      {...(isClickable && { role: 'button', tabIndex: 0 })}
      {...props}
    >
      {children}
    </div>
  )
}
