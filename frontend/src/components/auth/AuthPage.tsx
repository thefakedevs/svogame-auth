import { useEffect } from 'react'
import AuthCallbackPage from '../../pages/_AuthCallbackPage'
import AuthStartPage from '../../pages/_AuthStartPage'
import { useQuery } from '../../util/query'
import AuthErrorPage from './AuthErrorPage'

export default function AuthPage() {
  const query = useQuery()

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    if (window.location.hostname !== '127.0.0.1') {
      return
    }

    const nextUrl = new URL(window.location.href)
    nextUrl.hostname = 'localhost'
    window.location.replace(nextUrl.toString())
  }, [])

  if (typeof window !== 'undefined' && window.location.hostname === '127.0.0.1') {
    return null
  }

  if (query.get('error') || query.get('error_description')) {
    return <AuthErrorPage />
  }

  if (query.get('code')) {
    return <AuthCallbackPage />
  }

  return <AuthStartPage />
}
