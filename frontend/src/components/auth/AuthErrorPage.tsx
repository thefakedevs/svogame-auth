import { buildAuthUrl } from '../../routes/auth'
import ErrorState from '../ErrorState'
import { useQuery } from '../../util/query'

const authErrorTitles: Record<string, string> = {
  access_denied: 'Доступ не был предоставлен',
}

const authErrorDescriptions: Record<string, string> = {
  access_denied: 'Вход через Discord был отменен или отклонен. Попробуйте начать авторизацию заново.',
}

function resolveMessage(errorCode: string | null, errorDescription: string | null): string {
  if (errorDescription?.trim()) {
    return errorDescription
  }

  if (errorCode && authErrorDescriptions[errorCode]) {
    return authErrorDescriptions[errorCode]
  }

  return 'Не удалось завершить авторизацию через Discord. Попробуйте еще раз.'
}

export default function AuthErrorPage() {
  const query = useQuery()
  const errorCode = query.get('error')
  const errorDescription = query.get('error_description')
  const title = errorCode ? authErrorTitles[errorCode] ?? 'Авторизация не завершена' : 'Авторизация не завершена'

  return (
    <div className="page auth-error-page auth-pow-fullbleed">
      <div className="ui-kit-vhs" aria-hidden />
      <div className="ui-kit-page">
        <ErrorState
          title={title}
          message={resolveMessage(errorCode, errorDescription)}
          primaryActionLabel="Вернуться ко входу"
          onPrimaryAction={() => window.location.assign(buildAuthUrl())}
        />
      </div>
    </div>
  )
}
