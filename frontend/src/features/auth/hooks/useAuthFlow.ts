import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  fetchAuthorize,
  requestDiscordAuthInit,
  type DiscordAuthInitResponse,
} from '../../../api/auth'
import { toDisplayError } from '../../../api/http'
import { paths } from '../../../routes/paths'
import { navigateTo } from '../../../shared/navigation/history'
import { clearAuthSession, getAuthToken, getPowData, setPowData } from '../../../shared/session/auth-session'
import { validateTokenFormat } from '../../../shared/session/token'
import { solvePow, type PowProgressUpdate } from '../../../services/pow'
import { finishAuthDelivery } from '../lib/delivery'
import { normalizeLoopbackHost } from '../lib/navigation'
import { parsePollingData } from '../lib/polling'

export type AuthState =
  | {
      status: 'loading'
      powProgress?: PowProgressUpdate
      powComplexity?: number
      oauthUrl?: string
      errorMessage?: string
    }
  | {
      status: 'solving_pow'
      powProgress?: PowProgressUpdate
      powComplexity: number
      oauthUrl?: string
      errorMessage?: string
    }
  | {
      status: 'redirecting'
      oauthUrl: string
      powProgress?: PowProgressUpdate
      powComplexity?: number
      errorMessage?: string
    }
  | {
      status: 'error'
      errorMessage: string
      oauthUrl?: string
      powProgress?: PowProgressUpdate
      powComplexity?: number
    }

type AuthFlowOptions = {
  authHydrated: boolean
  discordCode: string | null
  pollingData: string | null
  returnUrl: string | null
  errorCode: string | null
  errorDescription: string | null
}

export function useAuthFlow(options: AuthFlowOptions) {
  const [state, setState] = useState<AuthState>({ status: 'loading' })
  const authHydrated = options.authHydrated

  useEffect(() => {
    normalizeLoopbackHost()
  }, [])

  useEffect(() => {
    if (!authHydrated || options.discordCode) {
      return
    }

    const existingToken = getAuthToken()
    const hasAuthParams = Boolean(options.returnUrl || options.pollingData)

    if (existingToken && validateTokenFormat(existingToken) && !hasAuthParams) {
      navigateTo(paths.profile, { replace: true })
      return
    }

    clearAuthSession()
  }, [authHydrated, options.discordCode, options.pollingData, options.returnUrl])

  const startAuth = useCallback(async () => {
    try {
      setState({ status: 'loading' })

      const authData: DiscordAuthInitResponse = options.pollingData
        ? parsePollingData(options.pollingData)
        : await requestDiscordAuthInit(options.returnUrl ?? paths.profile)

      setState({ status: 'solving_pow', powComplexity: authData.powComplexity })

      const updateProgress = (progress: PowProgressUpdate) => {
        setState((currentState) => ({
          ...currentState,
          powProgress: progress,
        }))
      }

      const powResult = await solvePow(authData.powPrefix, authData.powComplexity, updateProgress)
      setPowData({ solution: powResult, prefix: authData.powPrefix })
      setState({ status: 'redirecting', oauthUrl: authData.oauthUrl })
    } catch (error) {
      setState({
        status: 'error',
        errorMessage: toDisplayError(error, 'Не удалось инициализировать авторизацию.'),
      })
    }
  }, [options.pollingData, options.returnUrl])

  useEffect(() => {
    if (!authHydrated || options.errorCode || options.errorDescription || options.discordCode) {
      return
    }

    const timerId = window.setTimeout(() => {
      void startAuth()
    }, 0)

    return () => window.clearTimeout(timerId)
  }, [authHydrated, options.discordCode, options.errorCode, options.errorDescription, startAuth])

  useEffect(() => {
    if (!options.discordCode) {
      return
    }

    let cancelled = false

    async function completeAuth() {
      try {
        const auth = await fetchAuthorize(options.discordCode!, getPowData())
        if (cancelled) {
          return
        }

        setPowData(null)
        await finishAuthDelivery(auth)
      } catch (error) {
        if (cancelled) {
          return
        }

        setState({
          status: 'error',
          errorMessage: toDisplayError(error, 'Не удалось завершить авторизацию.'),
        })
      }
    }

    void completeAuth()

    return () => {
      cancelled = true
    }
  }, [options.discordCode])

  useEffect(() => {
    if (state.status !== 'redirecting') {
      return
    }

    const timerId = window.setTimeout(() => {
      navigateTo(state.oauthUrl)
    }, 500)

    return () => window.clearTimeout(timerId)
  }, [state])

  const retry = useCallback(() => {
    setState({ status: 'loading' })

    if (options.discordCode) {
      navigateTo(paths.auth, { replace: true })
      return
    }

    void startAuth()
  }, [options.discordCode, startAuth])

  return useMemo(
    () => ({
      state,
      retry,
    }),
    [retry, state],
  )
}
