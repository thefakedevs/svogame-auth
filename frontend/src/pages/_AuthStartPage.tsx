import { useEffect, useState } from 'react'
import AuthFlowStages from '../components/auth/AuthFlowStages'
import { type DiscordAuthInitResponse, requestDiscordAuthInit } from '../services/authApi'
import { useAuthStore } from '../store/authStore'
import { tokenManager } from '../services/tokenManager'
import { solvePow, type PowProgressUpdate } from '../services/pow.ts'
import { useQuery } from '../util/query.ts'
import { paths } from '../routes/paths'

interface InitState {
  status: 'loading' | 'solving_pow' | 'redirecting' | 'error'
  oauthUrl?: string
  errorMessage?: string
  powProgress?: PowProgressUpdate
  powComplexity?: number
}

export default function AuthStartPage() {
  const [state, setState] = useState<InitState>({ status: 'loading' })
  const authHydrated = useAuthStore((store) => store.hydrated)
  const setUser = useAuthStore((store) => store.setUser)
  const setPoWData = useAuthStore((store) => store.setPoWData)
  const query = useQuery()

  useEffect(() => {
    if (!authHydrated) {
      return
    }

    const existingToken = useAuthStore.getState().token
    const hasAuthParams = query.get('redirectUrl') || query.get('polling')

    if (existingToken && tokenManager.validateTokenFormat(existingToken) && !hasAuthParams) {
      window.location.assign(paths.profile)
      return
    }

    if (!query.get('code')) {
      setUser(null)
      setPoWData(null)
    }
  }, [authHydrated, query, setUser, setPoWData])

  useEffect(() => {
    if (!authHydrated) {
      return
    }

    let cancelled = false
    const returnUrl = query.get('redirectUrl')
    const pollingData = query.get('polling')

    async function initAuth() {
      try {
        let data = null as DiscordAuthInitResponse | null
        if (returnUrl) {
          data = await requestDiscordAuthInit(returnUrl)
        } else if (pollingData) {
          data = JSON.parse(atob(pollingData))
        }
        if (!data) {
          data = await requestDiscordAuthInit(paths.profile)
        }
        if (cancelled) return

        setState({ status: 'solving_pow', powComplexity: data.powComplexity })
        const updateProgress = (progress: PowProgressUpdate) => {
          if (cancelled) return
          setState((prevState) => ({
            ...prevState,
            powProgress: progress,
          }))
        }
        const powResult = await solvePow(data.powPrefix, data.powComplexity, updateProgress)

        useAuthStore.getState().setPoWData({ solution: powResult, prefix: data.powPrefix })
        setState({ status: 'redirecting', oauthUrl: data.oauthUrl })
      } catch (err) {
        if (cancelled) return
        console.error(err)
        const message = err instanceof Error ? err.message : 'Не удалось инициализировать авторизацию'
        setState({ status: 'error', errorMessage: message })
      }
    }

    initAuth().catch((err) => {
      if (cancelled) return
      const message = err instanceof Error ? err.message : 'Не удалось инициализировать авторизацию'
      setState({ status: 'error', errorMessage: message })
    })

    return () => {
      cancelled = true
    }
  }, [authHydrated, query])

  useEffect(() => {
    if (state.status !== 'redirecting' || !state.oauthUrl) return
    const url = state.oauthUrl
    const id = window.setTimeout(() => {
      window.location.assign(url)
    }, 500)
    return () => window.clearTimeout(id)
  }, [state.status, state.oauthUrl])

  return (
    <div className="page auth-start-page auth-pow-fullbleed">
      <AuthFlowStages
        stage={state}
        onRetryError={() => {
          setState({ status: 'loading' })
          window.location.reload()
        }}
      />
    </div>
  )
}
