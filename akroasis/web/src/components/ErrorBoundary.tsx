import { Component } from 'react'
import type { ReactNode, ErrorInfo } from 'react'
import { logError } from '../utils/errorLogger'

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    logError('react:boundary', error.message, {
      stack: error.stack,
      componentStack: errorInfo.componentStack,
    })
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback

      return (
        <div
          className="flex flex-col items-center justify-center p-8 text-center"
          style={{ color: 'rgb(var(--text-secondary))' }}
        >
          <p className="text-lg font-serif mb-2" style={{ color: 'rgb(var(--text-primary))' }}>
            Something went wrong
          </p>
          <p className="text-sm mb-4" style={{ color: 'rgb(var(--text-tertiary))' }}>
            {this.state.error?.message || 'An unexpected error occurred'}
          </p>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            className="px-4 py-2 rounded-lg text-sm transition-colors"
            style={{
              backgroundColor: 'rgb(var(--accent-primary) / 0.15)',
              color: 'rgb(var(--accent-primary))',
            }}
          >
            Try again
          </button>
        </div>
      )
    }

    return this.props.children
  }
}
