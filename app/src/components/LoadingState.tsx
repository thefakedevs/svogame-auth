import type { ReactNode } from 'react'

interface LoadingStateProps {
  title?: string
  message?: string
  children?: ReactNode
}

export default function LoadingState({ title, message, children }: LoadingStateProps) {
  return (
    <div className="card loading-state">
      {title && <h1 className="card-title">{title}</h1>}
      {message && <p className="card-text">{message}</p>}
      <div className="spinner" aria-hidden="true" />
      {children}
    </div>
  )
}

