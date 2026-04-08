import AuthCallbackPage from '../../pages/_AuthCallbackPage'
import AuthStartPage from '../../pages/_AuthStartPage'
import { useQuery } from '../../util/query'
import AuthErrorPage from './AuthErrorPage'

export default function AuthPage() {
  const query = useQuery()

  if (query.get('error') || query.get('error_description')) {
    return <AuthErrorPage />
  }

  if (query.get('code')) {
    return <AuthCallbackPage />
  }

  return <AuthStartPage />
}
