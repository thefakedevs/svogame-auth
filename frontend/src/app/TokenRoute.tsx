import { useEffect, useState } from 'react'
import { toDisplayError } from '../api/http'
import { getCurrentUser } from '../api/users'
import ErrorState from '../components/ErrorState'
import LoadingState from '../components/LoadingState'
import { buildAuthUrl } from '../routes/auth'
import { paths } from '../routes/paths'
import { navigateTo, replaceUrl } from '../shared/navigation/history'
import { storeAuthorizedSession } from '../shared/session/auth-session'
import { validateTokenFormat } from '../shared/session/token'

export default function TokenRoute() {
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    async function handleToken() {
      try {
        const params = new URLSearchParams(window.location.search)
        const token = params.get('token')

        if (!token) {
          navigateTo(buildAuthUrl(), { replace: true })
          return
        }

        if (!validateTokenFormat(token)) {
          throw new Error('Некорректный токен авторизации.')
        }

        const userData = await getCurrentUser(token)
        if (cancelled) return

        storeAuthorizedSession({
          token,
          user: {
            id: userData.id,
            username: userData.username,
            avatarUrl: userData.avatarUrl || '',
            isSuperuser: userData.isSuperuser,
          },
        })

        replaceUrl(window.location.pathname)
        navigateTo(paths.profile, { replace: true })
      } catch (cause) {
        if (cancelled) return
        setError(toDisplayError(cause, 'Ошибка при обработке токена авторизации.'))
      }
    }

    void handleToken()

    return () => {
      cancelled = true
    }
  }, [])

  if (error) {
    return (
      <div className="ui-kit-page page token-page">
        <ErrorState
          title="Ошибка авторизации"
          message={error}
          primaryActionLabel="Вернуться ко входу"
          onPrimaryAction={() => navigateTo(buildAuthUrl(), { replace: true })}
        />
      </div>
    )
  }

  return (
    <div className="ui-kit-page page token-page">
      <LoadingState title="Завершение авторизации" message="Сохраняем сессию и загружаем профиль..." />
    </div>
  )
}
