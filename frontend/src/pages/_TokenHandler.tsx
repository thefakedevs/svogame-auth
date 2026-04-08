import { buildAuthUrl } from '../routes/auth'
import { useEffect, useState } from 'react'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import { toDisplayError } from '../api/http'
import { getCurrentUser } from '../api/users'
import { paths } from '../routes/paths'
import { useAuthStore } from '../store/authStore'
import { setAuthToken } from '../util/tokenStorage'

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
          window.location.replace(buildAuthUrl())
          return
        }

        setAuthToken(token)

        const newUrl = window.location.pathname
        window.history.replaceState({}, document.title, newUrl)

        const userData = await getCurrentUser(token)
        if (cancelled) return

        const currentUser = useAuthStore.getState().user
        if (!currentUser || currentUser.id !== userData.id) {
          setUser({
            id: userData.id,
            username: userData.username,
            avatarUrl: userData.avatarUrl || '',
          })
        }

        window.location.replace(paths.profile)
      } catch (cause) {
        if (cancelled) return
        setError(toDisplayError(cause, 'Ошибка при обработке токена авторизации.'))
      }
    }

    void handleToken()

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
          primaryActionLabel="Вернуться ко входу"
          onPrimaryAction={() => window.location.replace(buildAuthUrl())}
        />
      </div>
    )
  }

  return (
    <div className="ui-kit-page page token-page">
      <LoadingState
        title="Завершение авторизации"
        message="Сохраняем сессию и загружаем профиль..."
      />
    </div>
  )
}
