import { useState, useRef, useEffect } from 'react'
import './NicknameEditor.css'

interface NicknameEditorProps {
  currentUsername: string
  onNicknameUpdate: (newNickname: string) => Promise<void>
  isLoading?: boolean
}

export default function NicknameEditor({ 
  currentUsername, 
  onNicknameUpdate, 
  isLoading = false 
}: NicknameEditorProps) {
  const [nickname, setNickname] = useState(currentUsername)
  const [isEditing, setIsEditing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [successMessage, setSuccessMessage] = useState<string | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // Reset nickname when currentUsername changes
  useEffect(() => {
    setNickname(currentUsername)
  }, [currentUsername])

  // Focus input when entering edit mode
  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus()
      inputRef.current.select()
    }
  }, [isEditing])

  const handleEditClick = () => {
    setIsEditing(true)
    setError(null)
    setSuccessMessage(null)
  }

  const handleCancel = () => {
    setNickname(currentUsername) // Restore original value
    setIsEditing(false)
    setError(null)
    setSuccessMessage(null)
  }

  const validateNickname = (value: string): string | null => {
    const trimmed = value.trim()
    
    if (trimmed === '') {
      return 'Никнейм не может быть пустым'
    }
    if (trimmed !== value) {
      return 'Никнейм не может начинаться или заканчиваться пробелами'
    }
    if (trimmed.length < 3 || trimmed.length > 16) {
      return 'Никнейм должен содержать от 3 до 16 символов'
    }
    if (!/^[a-zA-Z0-9_]+$/.test(trimmed)) {
      return 'Никнейм может содержать только буквы, цифры и символ подчеркивания'
    }
  
    return null
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    
    const validationError = validateNickname(nickname)
    if (validationError) {
      setError(validationError)
      return
    }

    // Don't submit if nickname hasn't changed
    if (nickname === currentUsername) {
      setIsEditing(false)
      return
    }

    try {
      setError(null)
      await onNicknameUpdate(nickname)
      setIsEditing(false)
      setSuccessMessage('Никнейм успешно обновлен!')
      
      // Clear success message after 3 seconds
      setTimeout(() => {
        setSuccessMessage(null)
      }, 3000)
    } catch (error) {
      // Restore previous value on error
      setNickname(currentUsername)
      setError(error instanceof Error ? error.message : 'Не удалось обновить никнейм')
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      handleCancel()
    }
  }

  if (!isEditing) {
    return (
      <div className="nickname-editor">
        <div className="nickname-display">
          <h2 className="current-nickname">{currentUsername}</h2>
          <button 
            className="edit-button"
            onClick={handleEditClick}
            disabled={isLoading}
            aria-label="Редактировать никнейм"
          >
            <svg className="edit-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
              <path d="m18.5 2.5 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
            </svg>
          </button>
        </div>
        
        {successMessage && (
          <div className="success-message">
            <svg className="success-icon" viewBox="0 0 24 24" fill="currentColor">
              <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
            </svg>
            {successMessage}
          </div>
        )}
      </div>
    )
  }

  return (
    <div className="nickname-editor">
      <form className="nickname-form" onSubmit={handleSubmit}>
        <div className="input-container">
          <input
            ref={inputRef}
            type="text"
            value={nickname}
            onChange={(e) => setNickname(e.target.value)}
            onKeyDown={handleKeyDown}
            className={`nickname-input ${error ? 'error' : ''}`}
            placeholder="Введите никнейм (3-16 символов, a-z, 0-9, _)"
            disabled={isLoading}
            maxLength={16}
          />
          
          <div className="button-group">
            <button
              type="submit"
              className="save-button"
              disabled={isLoading || validateNickname(nickname) !== null}
            >
              {isLoading ? (
                <div className="loading-spinner" />
              ) : (
                <svg className="save-icon" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                </svg>
              )}
            </button>
            
            <button
              type="button"
              className="cancel-button"
              onClick={handleCancel}
              disabled={isLoading}
            >
              <svg className="cancel-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
        </div>
        
        <div className="validation-hint">
          <svg className="info-icon" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
          </svg>
          Правила: 3-16 символов, только буквы, цифры и _
        </div>
        
        {error && (
          <div className="error-message">
            <svg className="error-icon" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
            </svg>
            {error}
          </div>
        )}
      </form>
    </div>
  )
}