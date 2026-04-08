import type { ReactNode } from 'react'
import './LoadingState.css'

interface LoadingStateProps {
  title?: string
  message?: string
  children?: ReactNode
}

export default function LoadingState({ title, message, children }: LoadingStateProps) {
  return (
    <div className="card loading-state">
      <div className="loading-icon-container">
        <span className="ui-spinner ui-spinner-lg" role="status" aria-label="Загрузка">
          <span className="ui-spinner-track" aria-hidden>
            <span className="ui-spinner-orb" />
            <span className="ui-spinner-orb" />
            <span className="ui-spinner-orb" />
            <span className="ui-spinner-orb" />
          </span>
        </span>
      </div>
      <div className="loading-content">
        {title && <h1 className="card-title">{title}</h1>}
        {message && <p className="card-text">{message}</p>}
      </div>
      {children && <div className="loading-children">{children}</div>}
    </div>
  )
}

