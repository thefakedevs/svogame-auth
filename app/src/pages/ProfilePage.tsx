import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { getCurrentUser, type User } from '../services/userApi'
import { removeAuthToken, isAuthenticated } from '../util/tokenStorage'
import { useAuthStore } from '../store/authStore'
import LoadingState from '../components/LoadingState'
import ErrorState from '../components/ErrorState'
import UserProfileCard from '../components/UserProfileCard'
import './ProfilePage.css'

type PageState = 
    | { status: 'loading' }
    | { status: 'loaded', user: User }
    | { status: 'error', errorMessage: string }

export default function ProfilePage() {
    const [state, setState] = useState<PageState>({ status: 'loading' })
    const navigate = useNavigate()
    const { setUser } = useAuthStore()

    useEffect(() => {
        // Проверяем авторизацию и загружаем данные
        if (!isAuthenticated()) {
            navigate('/auth', { replace: true })
            return
        }

        let cancelled = false

        async function loadUser() {
            try {
                const userData = await getCurrentUser()
                if (cancelled) return

                setState({ status: 'loaded', user: userData })
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
                    navigate('/auth', { replace: true })
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
    }, []) // Пустой массив зависимостей - выполняется только при монтировании

    const handleEditNickname = () => {
        navigate('/edit-nickname')
    }

    const handleLogout = () => {
        removeAuthToken()
        setUser(null)
        navigate('/auth', { replace: true })
    }

    const handleRetry = () => {
        setState({ status: 'loading' })
        window.location.reload()
    }

    if (state.status === 'loading') {
        return (
            <LoadingState
                title="Загрузка профиля"
                message="Получаем данные вашего профиля..."
            />
        )
    }

    if (state.status === 'error') {
        return (
            <ErrorState
                title="Ошибка загрузки"
                message={state.errorMessage}
                primaryActionLabel="Повторить"
                onPrimaryAction={handleRetry}
            />
        )
    }

    const { user } = state

    return (
        <div className="profile-page">
            <div className="profile-container">
                <h1>Ваш профиль</h1>

                <div className="current-profile">
                    <UserProfileCard user={{
                        id: user.id,
                        username: user.username,
                        avatarUrl: user.avatarUrl || ''
                    }} />
                </div>

                <div className="profile-actions">
                    <button 
                        className="btn primary"
                        onClick={handleEditNickname}
                    >
                        <svg className="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                            <path d="m18.5 2.5 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
                        </svg>
                        Изменить никнейм
                    </button>
                    <button 
                        className="btn secondary"
                        onClick={handleLogout}
                    >
                        <svg className="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>
                            <polyline points="16,17 21,12 16,7"/>
                            <line x1="21" y1="12" x2="9" y2="12"/>
                        </svg>
                        Выйти
                    </button>
                </div>
            </div>
        </div>
    )
}