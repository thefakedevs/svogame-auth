import { useCallback, useEffect, useState } from 'react'
import AuthFlowStages, { type AuthFlowStageState } from '../components/auth/AuthFlowStages'
import { type AuthorizationCallbackResponse, fetchAuthorize } from '../services/authApi'
import { useAuthStore } from '../store/authStore'
import { useQuery } from '../util/query.ts'
import { paths } from '../routes/paths'
import { tokenManager } from '../services/tokenManager'

interface CallbackState {
  status: 'loadingProfile' | 'success' | 'error' | 'delivering'
  user?: AuthorizationCallbackResponse
  errorMessage?: string
}

function toFlowStage(state: CallbackState): AuthFlowStageState {
  if (state.status === 'error') {
    return {
      status: 'error',
      errorMessage: state.errorMessage ?? 'Не удалось завершить авторизацию',
    }
  }

  if (state.status === 'loadingProfile') {
    return { status: 'loading' }
  }

  return { status: 'redirecting' }
}

export default function AuthCallbackPage() {
  const query = useQuery()
  const [state, setState] = useState<CallbackState>({ status: 'loadingProfile' })
  const { setUser, setPoWData, setToken } = useAuthStore()

  const persistAuth = useCallback(async (auth: AuthorizationCallbackResponse) => {
    setUser(auth.user)
    setToken(auth.accessToken)
    await tokenManager.storeToken(auth.accessToken)
  }, [setToken, setUser])

  const finishDelivery = useCallback(async function() {
    if (!state.user) return

    if (state.user.deliveryMethod === 'polling') {
      await persistAuth(state.user)
      window.location.replace(paths.profile)
      return
    }

    try {
      await persistAuth(state.user)

      const parsed = new URL(state.user.deliveryTarget, window.location.origin)

      if (parsed.origin === window.location.origin) {
        window.location.replace(parsed.pathname + parsed.search)
        return
      }

      parsed.searchParams.set('token', state.user.accessToken)
      window.location.href = parsed.toString()
    } catch {
      const tokenParam = state.user.deliveryTarget.includes('?') ? '&' : '?'
      window.location.href = `${state.user.deliveryTarget}${tokenParam}token=${encodeURIComponent(state.user.accessToken)}`
    }
  }, [persistAuth, state.user])

  useEffect(() => {
    if ((state.status === 'success' || state.status === 'delivering') && state.user) {
      const t = window.setTimeout(() => {
        finishDelivery()
      }, 50)
      return () => window.clearTimeout(t)
    }
  }, [finishDelivery, state.status, state.user])

  useEffect(() => {
    let cancelled = false

    async function loadProfile() {
      try {
        const code = query.get('code') ?? ''

        let pow = useAuthStore.getState().powData
        if (!pow) {
          try {
            const stored = sessionStorage.getItem('pow-data')
            if (stored) {
              pow = JSON.parse(stored)
            }
          } catch {
            // ignore parse errors
          }
        }

        const data = await fetchAuthorize(code, pow)
        if (cancelled) return

        setPoWData(null)

        if (data.deliveryMethod === 'redirect') {
          setState({ status: 'success', user: data })
          return
        }

        if (data.deliveryMethod === 'polling') {
          setState({ status: 'delivering', user: data })
          return
        }

        throw new Error(`Unknown auth delivery method: ${data.deliveryMethod}`)
      } catch (err) {
        if (cancelled) return
        const message = err instanceof Error ? err.message : 'Не удалось загрузить профиль'
        setState({ status: 'error', errorMessage: message })
      }
    }

    loadProfile()

    return () => {
      cancelled = true
    }
  }, [query, setPoWData])

  return (
    <div className="page auth-callback-page auth-pow-fullbleed">
      <AuthFlowStages
        stage={toFlowStage(state)}
        onRetryError={() => {
          setState({ status: 'loadingProfile' })
          window.location.replace(paths.auth)
        }}
      />
    </div>
  )
}
