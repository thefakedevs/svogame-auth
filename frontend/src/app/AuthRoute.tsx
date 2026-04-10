import AuthErrorPage from '../components/auth/AuthErrorPage'
import AuthFlowStages from '../components/auth/AuthFlowStages'
import { useAuthFlow } from '../features/auth/hooks/useAuthFlow'
import { useAuthStore } from '../store/authStore'
import { useQuery } from '../util/query'

export default function AuthRoute() {
  const query = useQuery()
  const authHydrated = useAuthStore((store) => store.hydrated)

  const returnUrl = query.get('redirectUrl')
  const pollingData = query.get('polling')
  const errorCode = query.get('error')
  const errorDescription = query.get('error_description')
  const discordCode = query.get('code')

  const { state, retry } = useAuthFlow({
    authHydrated,
    discordCode,
    pollingData,
    returnUrl,
    errorCode,
    errorDescription,
  })

  if (typeof window !== 'undefined' && window.location.hostname === '127.0.0.1') {
    return null
  }

  if (errorCode || errorDescription) {
    return <AuthErrorPage />
  }

  return (
    <div className="page auth-page auth-pow-fullbleed">
      <AuthFlowStages stage={state} onRetryError={retry} />
    </div>
  )
}
