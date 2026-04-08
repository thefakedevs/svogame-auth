import { useEffect, useState } from 'react'
import { getCurrentUser, updateNickname, type UserResponse } from '../services/userApi'
import { useAuthStore } from '../store/authStore'
import { isAuthenticated, removeAuthToken } from '../util/tokenStorage'
import LoadingState from '../components/LoadingState'
import ErrorState from '../components/ErrorState'
import { paths } from '../routes/paths'
import './UserEditPage.css'

type PageState = 
    | { status: 'loading' }
    | { status: 'loaded', user: UserResponse }
    | { status: 'updating' }
    | { status: 'success', user: UserResponse }
    | { status: 'error', errorMessage: string }

export default function UserEditPage() {
    const [state, setState] = useState<PageState>({ status: 'loading' })
    const [nickname, setNickname] = useState('')
    const [validationError, setValidationError] = useState('')
    const [authChecked, setAuthChecked] = useState(false)
    const { user: authUser, setUser } = useAuthStore()

    // Проверяем авторизацию при загрузке компонента
    useEffect(() => {
        if (authChecked) return // Предотвращаем повторные проверки
        
        if (!isAuthenticated()) {
            window.location.replace(paths.auth)
            return
        }
        setAuthChecked(true)
    }, [authChecked])

    useEffect(() => {
        // Если не авторизован или еще не проверили авторизацию, не загружаем данные
        if (!isAuthenticated() || !authChecked) {
            return
        }

        let cancelled = false

        async function loadUser() {
            try {
                // Get token from auth store
                const token = useAuthStore.getState().token
                if (!token) {
                    throw new Error('No authentication token available')
                }

                const userData = await getCurrentUser(token)
                if (cancelled) return

                setState({ status: 'loaded', user: userData })
                setNickname(userData.username)
            } catch (err) {
                if (cancelled) return
                
                // Если ошибка связана с авторизацией, очищаем токен и перенаправляем
                if (err instanceof Error && (
                    err.message.includes('токен') || 
                    err.message.includes('авторизации') ||
                    err.message.includes('401') ||
                    err.message.includes('403')
                )) {
                    removeAuthToken()
                    setUser(null)
                    window.location.replace(paths.auth)
                    return
                }
                
                const message = err instanceof Error ? err.message : 'Ошибка при загрузке данных пользователя'
                setState({ status: 'error', errorMessage: message })
            }
        }

        loadUser()

        return () => {
            cancelled = true
        }
    }, [authChecked, setUser]) // Добавляем необходимые зависимости

    const validateNickname = (value: string): string => {
        if (!value.trim()) {
            return 'Никнейм не может быть пустым'
        }
        
        if (value.length < 2) {
            return 'Никнейм должен содержать минимум 2 символа'
        }
        
        if (value.length > 16) {
            return 'Никнейм не может содержать более 16 символов'
        }
        
        const nicknameRegex = /^[a-zA-Z0-9_]+$/
        if (!nicknameRegex.test(value)) {
            return 'Никнейм может содержать только буквы, цифры и подчеркивания'
        }
        
        return ''
    }

    const handleNicknameChange = (value: string) => {
        setNickname(value)
        const error = validateNickname(value)
        setValidationError(error)
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        
        const error = validateNickname(nickname)
        if (error) {
            setValidationError(error)
            return
        }

        if (state.status === 'loaded' && nickname === state.user.username) {
            setValidationError('Новый никнейм должен отличаться от текущего')
            return
        }

        setState({ status: 'updating' })
        setValidationError('')

        try {
            const token = useAuthStore.getState().token
            if (!token) {
                throw new Error('No authentication token available')
            }

            const updatedUser = await updateNickname(token, nickname)
            
            // Обновляем пользователя в auth store
            if (authUser) {
                setUser({
                    ...authUser,
                    username: updatedUser.username
                })
            }
            
            setState({ status: 'success', user: updatedUser })
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Ошибка при обновлении никнейма'
            setState({ status: 'error', errorMessage: message })
        }
    }

    const handleRetry = () => {
        setState({ status: 'loading' })
        window.location.reload()
    }

    const handleBackToProfile = () => {
        window.location.assign(paths.profile)
    }

    if (state.status === 'loading') {
        return (
            <div className="ui-kit-page user-edit-page">
                <LoadingState
                    title="Загрузка профиля"
                    message="Получаем данные вашего профиля..."
                />
            </div>
        )
    }

    if (state.status === 'error') {
        return (
            <div className="ui-kit-page user-edit-page">
                <ErrorState
                    title="Ошибка загрузки"
                    message={state.errorMessage}
                    primaryActionLabel="Повторить"
                    onPrimaryAction={handleRetry}
                />
            </div>
        )
    }

    if (state.status === 'success') {
        return (
            <div className="ui-kit-page user-edit-page">
                <div className="card success-container">
                    <div className="success-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
                            <polyline points="22,4 12,14.01 9,11.01"/>
                        </svg>
                    </div>
                    <h1 className="card-title">Никнейм успешно обновлен!</h1>
                    <p>Ваш новый никнейм: <strong>{state.user.username}</strong></p>
                    
                    <button 
                        className="btn primary"
                        onClick={handleBackToProfile}
                    >
                        Вернуться к профилю
                    </button>
                </div>
            </div>
        )
    }

    const user = state.status === 'loaded' ? state.user : null
    const isUpdating = state.status === 'updating'

    return (
        <div className="ui-kit-page user-edit-page">
            <div className="edit-container card">
                <h1 className="card-title">Изменение никнейма</h1>
                
                {user && (
                    <div className="ui-alert ui-alert-info current-profile">
                        <span className="ui-alert-icon" aria-hidden>ℹ</span>
                        <span>Текущий никнейм: <strong>{user.username}</strong></span>
                    </div>
                )}

                <form onSubmit={handleSubmit} className="nickname-form">
                    <div className={`ui-field form-group ${validationError ? 'ui-field-error' : ''}`}>
                        <label className="ui-label" htmlFor="nickname">Новый никнейм</label>
                        <input
                            id="nickname"
                            className="ui-input"
                            type="text"
                            value={nickname}
                            onChange={(e) => handleNicknameChange(e.target.value)}
                            disabled={isUpdating}
                            placeholder="Введите новый никнейм"
                            maxLength={16}
                        />
                        {validationError && (
                            <div className="validation-error">
                                {validationError}
                            </div>
                        )}
                        <div className="ui-hint form-help">
                            Никнейм должен содержать от 2 до 16 символов и может включать только буквы, цифры и подчеркивания.
                        </div>
                    </div>

                    <div className="form-actions">
                        <button
                            type="button"
                            className="btn"
                            onClick={handleBackToProfile}
                            disabled={isUpdating}
                        >
                            Отмена
                        </button>
                        <button
                            type="submit"
                            className="btn primary"
                            disabled={isUpdating || !!validationError || !nickname.trim()}
                        >
                            {isUpdating ? (
                                <>
                                    <div className="btn-spinner"></div>
                                    Обновление...
                                </>
                            ) : (
                                'Сохранить'
                            )}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    )
}
