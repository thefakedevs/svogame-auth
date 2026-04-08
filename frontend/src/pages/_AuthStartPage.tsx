import { useCallback, useEffect, useState } from 'react'
import AuthStartCard from '../components/auth/AuthStartCard'
import AuthFlowStages from '../components/auth/AuthFlowStages'
import { type DiscordAuthInitResponse, requestDiscordAuthInit } from '../api/auth'
import { toDisplayError } from '../api/http'
import { paths } from '../routes/paths'
import { solvePow, type PowProgressUpdate } from '../services/pow.ts'
import { tokenManager } from '../services/tokenManager'
import { useAuthStore } from '../store/authStore'
import { useQuery } from '../util/query.ts'

interface InitState {
  status: 'idle' | 'loading' | 'solving_pow' | 'redirecting' | 'error'
  oauthUrl?: string
  errorMessage?: string
  powProgress?: PowProgressUpdate
  powComplexity?: number
}

export default function AuthStartPage() {
  const [state, setState] = useState<InitState>({ status: 'idle' })
  const authHydrated = useAuthStore((store) => store.hydrated)
  const setUser = useAuthStore((store) => store.setUser)
  const setPoWData = useAuthStore((store) => store.setPoWData)
  const query = useQuery()
  const returnUrl = query.get('redirectUrl')
  const pollingData = query.get('polling')
  const shouldAutoStart = Boolean(returnUrl || pollingData)

  useEffect(() => {
    if (!authHydrated) {
      return
    }

    const existingToken = useAuthStore.getState().token
    const hasAuthParams = returnUrl || pollingData

    if (existingToken && tokenManager.validateTokenFormat(existingToken) && !hasAuthParams) {
      window.location.assign(paths.profile)
      return
    }

    if (!query.get('code')) {
      setUser(null)
      setPoWData(null)
    }
  }, [authHydrated, pollingData, returnUrl, query, setUser, setPoWData])

  const startAuth = useCallback(async () => {
    let data: DiscordAuthInitResponse | null = null

    try {
      setState({ status: 'loading' })

      if (returnUrl) {
        data = await requestDiscordAuthInit(returnUrl)
      } else if (pollingData) {
        data = JSON.parse(atob(pollingData))
      } else {
        data = await requestDiscordAuthInit(paths.profile)
      }

      setState({ status: 'solving_pow', powComplexity: data.powComplexity })

      const updateProgress = (progress: PowProgressUpdate) => {
        setState((currentState) => ({
          ...currentState,
          powProgress: progress,
        }))
      }

      const powResult = await solvePow(data.powPrefix, data.powComplexity, updateProgress)

      useAuthStore.getState().setPoWData({ solution: powResult, prefix: data.powPrefix })
      setState({ status: 'redirecting', oauthUrl: data.oauthUrl })
    } catch (error) {
      setState({
        status: 'error',
        errorMessage: toDisplayError(error, 'Не удалось инициализировать авторизацию.'),
      })
    }
  }, [pollingData, returnUrl])

  useEffect(() => {
    if (!authHydrated || !shouldAutoStart) {
      return
    }

    const timerId = window.setTimeout(() => {
      void startAuth()
    }, 0)

    return () => window.clearTimeout(timerId)
  }, [authHydrated, shouldAutoStart, startAuth])

  useEffect(() => {
    if (state.status !== 'redirecting' || !state.oauthUrl) return

    const timerId = window.setTimeout(() => {
      window.location.assign(state.oauthUrl!)
    }, 500)

    return () => window.clearTimeout(timerId)
  }, [state.oauthUrl, state.status])

  return (
    state.status === 'idle' || (!shouldAutoStart && state.status === 'error') ? (
      <AuthStartCard
        onStart={() => void startAuth()}
        errorMessage={state.status === 'error' ? state.errorMessage : undefined}
        isLoading={state.status === 'loading'}
      />
    ) : (
      <div className="page auth-start-page auth-pow-fullbleed">
        <AuthFlowStages
          stage={state}
          onRetryError={() => {
            if (shouldAutoStart) {
              setState({ status: 'loading' })
              void startAuth()
              return
            }
            setState({ status: 'idle' })
          }}
        />
      </div>
    )
  )
}
