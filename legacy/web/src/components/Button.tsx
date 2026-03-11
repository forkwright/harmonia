import { forwardRef } from 'react'
import type { ButtonHTMLAttributes } from 'react'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost'
  size?: 'sm' | 'md' | 'lg'
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'primary', size = 'md', className = '', children, style, ...props }, ref) => {
    const sizes = {
      sm: 'px-3 py-1.5 text-sm',
      md: 'px-4 py-2 text-base',
      lg: 'px-6 py-3 text-lg',
    }

    const variantStyles: Record<string, React.CSSProperties> = {
      primary: {
        backgroundColor: 'rgb(var(--accent-primary))',
        color: '#fff',
      },
      secondary: {
        backgroundColor: 'rgb(var(--surface-sunken))',
        color: 'rgb(var(--text-primary))',
      },
      ghost: {
        backgroundColor: 'transparent',
        color: 'rgb(var(--text-secondary))',
      },
    }

    return (
      <button
        ref={ref}
        className={`inline-flex items-center justify-center font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed ${sizes[size]} ${className}`}
        style={{
          ...variantStyles[variant],
          ...style,
        }}
        {...props}
      >
        {children}
      </button>
    )
  }
)

Button.displayName = 'Button'
