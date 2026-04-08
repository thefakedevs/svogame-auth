import { useEffect, useState } from 'react'
import { setAuthToken } from '../util/tokenStorage'
import { getCurrentUser } from '../services/userApi'
import { useAuthStore } from '../store/authStore'
import LoadingState from '../components/LoadingState'
import ErrorState from '../components/ErrorState'
import { paths } from '../routes/paths'

export default function TokenHandler() {
    const { setUser } = useAuthStore()
    const [error, setError] = useState<string | null>(null)

    useEffect(() => {
        let cancelled = false

        async function handleToken() {
            try {
                const params = new URLSearchParams(window.location.search)
                const token = params.get('token')

                if (!token) {
                    // No token found, redirect to auth
                    window.location.replace(paths.auth)
                    return
                }

                // Сохраняем токен
                setAuthToken(token)

                // Убираем токен из URL, сохраняя чистый путь
                const newUrl = window.location.pathname
                window.history.replaceState({}, document.title, newUrl)

                // Получаем данные пользователя с новым токеном
                const userData = await getCurrentUser(token)
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
                window.location.replace(paths.profile)
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
    }, [setUser])

    if (error) {
        return (
            <div className="ui-kit-page page token-page">
                <ErrorState
                    title="Ошибка авторизации"
                    message={error}
                    primaryActionLabel="Вернуться к авторизации"
                    onPrimaryAction={() => window.location.replace(paths.auth)}
                />
            </div>
        )
    }

    return (
        <div className="ui-kit-page page token-page">
            <LoadingState
                title="Завершение авторизации"
                message="Сохраняем данные авторизации и получаем профиль..."
            />
        </div>
    )
}
