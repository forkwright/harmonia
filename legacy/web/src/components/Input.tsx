import { forwardRef, useId } from 'react'
import type { InputHTMLAttributes } from 'react'

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string
  error?: string
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, className = '', id: providedId, ...props }, ref) => {
    const generatedId = useId()
    const inputId = providedId || generatedId

    return (
      <div className="w-full">
        {label && (
          <label
            htmlFor={inputId}
            className="block text-sm font-medium mb-1"
            style={{ color: 'rgb(var(--text-secondary))' }}
          >
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={`w-full px-4 py-2 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-offset-1 ${className}`}
          style={{
            backgroundColor: 'rgb(var(--surface-sunken))',
            border: '1px solid rgb(var(--border-default))',
            color: 'rgb(var(--text-primary))',
          }}
          {...props}
        />
        {error && (
          <p className="mt-1 text-sm" style={{ color: 'rgb(var(--error-text))' }}>{error}</p>
        )}
      </div>
    )
  }
)

Input.displayName = 'Input'
