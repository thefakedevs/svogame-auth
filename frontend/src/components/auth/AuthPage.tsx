import AuthCallbackPage from '../../pages/_AuthCallbackPage'
import AuthStartPage from '../../pages/_AuthStartPage'
import { useQuery } from '../../util/query'

export default function AuthPage() {
  const query = useQuery()
  return query.get('code') ? <AuthCallbackPage /> : <AuthStartPage />
}
