import { useEffect, useState } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
import { setAuthToken } from '../util/tokenStorage'
import { getCurrentUser } from '../services/userApi'
import { useAuthStore } from '../store/authStore'
import LoadingState from '../components/LoadingState'
import ErrorState from '../components/ErrorState'

export default function TokenHandler() {
    const navigate = useNavigate()
    const location = useLocation()
    const { setUser } = useAuthStore()
    const [error, setError] = useState<string | null>(null)

    useEffect(() => {
        let cancelled = false

        async function handleToken() {
            try {
                const params = new URLSearchParams(location.search)
                const token = params.get('token')

                if (!token) {
                    // No token found, redirect to auth
                    navigate('/auth', { replace: true })
                    return
                }

                // Сохраняем токен
                setAuthToken(token)

                // Убираем токен из URL, сохраняя чистый путь
                const newUrl = window.location.pathname
                window.history.replaceState({}, document.title, newUrl)

                // Получаем данные пользователя с новым токеном
                const userData = await getCurrentUser()
                if (cancelled) return

                // Обновляем store с данными пользователя только если данные изменились
                const currentUser = useAuthStore.getState().user
                if (!currentUser || currentUser.id !== userData.id) {
                    setUser({
                        id: userData.id,
                        username: userData.username,
                        avatarUrl: userData.avatarUrl || ''
                    })
                }

                // Перенаправляем на страницу профиля
                navigate('/profile', { replace: true })
            } catch (err) {
                if (cancelled) return
                const message = err instanceof Error ? err.message : 'Ошибка при обработке токена'
                setError(message)
            }
        }

        handleToken()

        return () => {
            cancelled = true
        }
    }, [location.search, navigate, setUser])

    if (error) {
        return (
            <ErrorState
                title="Ошибка авторизации"
                message={error}
                primaryActionLabel="Вернуться к авторизации"
                onPrimaryAction={() => navigate('/auth', { replace: true })}
            />
        )
    }

    return (
        <LoadingState
            title="Завершение авторизации"
            message="Сохраняем данные авторизации и получаем профиль..."
        />
    )
}