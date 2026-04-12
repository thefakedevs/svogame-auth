import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  completeRegistration,
  fetchAuthorize,
  requestDiscordAuthInit,
  type DiscordAuthInitResponse,
} from '../../../api/auth'
import { toDisplayError } from '../../../api/http'
import { paths } from '../../../routes/paths'
import { navigateTo, replaceUrl } from '../../../shared/navigation/history'
import { clearAuthSession, getAuthToken, getPowData, setPowData } from '../../../shared/session/auth-session'
import { validateTokenFormat } from '../../../shared/session/token'
import { solvePow, type PowProgressUpdate } from '../../../services/pow'
import { finishAuthDelivery } from '../lib/delivery'
import { normalizeLoopbackHost } from '../lib/navigation'
import { parsePollingData } from '../lib/polling'

type AuthFlowState =
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

type AuthTermsState =
  | {
      status: 'awaiting_terms'
      registrationToken: string
      errorMessage?: string
    }
  | {
      status: 'submitting_terms'
      registrationToken: string
      errorMessage?: string
    }

export type AuthState = AuthFlowState | AuthTermsState

type AuthFlowOptions = {
  authHydrated: boolean
  discordCode: string | null
  pollingData: string | null
  returnUrl: string | null
  errorCode: string | null
  errorDescription: string | null
  registrationToken: string | null
}

function registrationUrl(token: string) {
  return `${paths.auth}?registrationToken=${encodeURIComponent(token)}`
}

export function useAuthFlow(options: AuthFlowOptions) {
  const [state, setState] = useState<AuthState>(() =>
    options.registrationToken
      ? { status: 'awaiting_terms', registrationToken: options.registrationToken }
      : { status: 'loading' },
  )
  const authHydrated = options.authHydrated

  useEffect(() => {
    normalizeLoopbackHost()
  }, [])

  useEffect(() => {
    if (!authHydrated || options.discordCode) {
      return
    }

    const existingToken = getAuthToken()
    const hasAuthParams = Boolean(
      options.returnUrl || options.pollingData || options.registrationToken,
    )

    if (existingToken && validateTokenFormat(existingToken) && !hasAuthParams) {
      navigateTo(paths.profile, { replace: true })
      return
    }

    clearAuthSession()
  }, [
    authHydrated,
    options.discordCode,
    options.pollingData,
    options.registrationToken,
    options.returnUrl,
  ])

  useEffect(() => {
    const registrationToken = options.registrationToken

    if (!registrationToken) {
      return
    }

    setState((currentState) => {
      if (
        (currentState.status === 'awaiting_terms' || currentState.status === 'submitting_terms') &&
        currentState.registrationToken === registrationToken
      ) {
        return currentState
      }

      return {
        status: 'awaiting_terms',
        registrationToken,
      }
    })
  }, [options.registrationToken])

  const startAuth = useCallback(async () => {
    try {
      setState({ status: 'loading' })

      const authData: DiscordAuthInitResponse = options.pollingData
        ? parsePollingData(options.pollingData)
        : await requestDiscordAuthInit(options.returnUrl ?? paths.profile)

      setState({ status: 'solving_pow', powComplexity: authData.powComplexity })

      const updateProgress = (progress: PowProgressUpdate) => {
        setState((currentState) => {
          if (currentState.status !== 'solving_pow') {
            return currentState
          }

          return {
            ...currentState,
            powProgress: progress,
          }
        })
      }

      const powResult = await solvePow(authData.powPrefix, authData.powComplexity, updateProgress)
      setPowData({ solution: powResult, prefix: authData.powPrefix })
      setState({ status: 'redirecting', oauthUrl: authData.oauthUrl })
    } catch (error) {
      setState({
        status: 'error',
        errorMessage: toDisplayError(error, 'РќРµ СѓРґР°Р»РѕСЃСЊ РёРЅРёС†РёР°Р»РёР·РёСЂРѕРІР°С‚СЊ Р°РІС‚РѕСЂРёР·Р°С†РёСЋ.'),
      })
    }
  }, [options.pollingData, options.returnUrl])

  useEffect(() => {
    if (
      !authHydrated ||
      options.errorCode ||
      options.errorDescription ||
      options.discordCode ||
      options.registrationToken
    ) {
      return
    }

    const timerId = window.setTimeout(() => {
      void startAuth()
    }, 0)

    return () => window.clearTimeout(timerId)
  }, [
    authHydrated,
    options.discordCode,
    options.errorCode,
    options.errorDescription,
    options.registrationToken,
    startAuth,
  ])

  useEffect(() => {
    if (!options.discordCode || options.registrationToken) {
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

        if (auth.status === 'terms_required') {
          replaceUrl(registrationUrl(auth.registrationToken))
          setState({
            status: 'awaiting_terms',
            registrationToken: auth.registrationToken,
          })
          return
        }

        await finishAuthDelivery(auth)
      } catch (error) {
        if (cancelled) {
          return
        }

        setState({
          status: 'error',
          errorMessage: toDisplayError(error, 'РќРµ СѓРґР°Р»РѕСЃСЊ Р·Р°РІРµСЂС€РёС‚СЊ Р°РІС‚РѕСЂРёР·Р°С†РёСЋ.'),
        })
      }
    }

    void completeAuth()

    return () => {
      cancelled = true
    }
  }, [options.discordCode, options.registrationToken])

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

    if (options.discordCode || options.registrationToken) {
      navigateTo(paths.auth, { replace: true })
      return
    }

    void startAuth()
  }, [options.discordCode, options.registrationToken, startAuth])

  const acceptTerms = useCallback(async () => {
    const registrationToken =
      state.status === 'awaiting_terms' || state.status === 'submitting_terms'
        ? state.registrationToken
        : options.registrationToken

    if (!registrationToken) {
      setState({
        status: 'error',
        errorMessage: 'РЎРµСЃСЃРёСЏ СЂРµРіРёСЃС‚СЂР°С†РёРё РёСЃС‚РµРєР»Р°. РќР°С‡РЅРёС‚Рµ РІС…РѕРґ Р·Р°РЅРѕРІРѕ.',
      })
      return
    }

    setState({
      status: 'submitting_terms',
      registrationToken,
    })

    try {
      const auth = await completeRegistration(registrationToken)
      await finishAuthDelivery(auth)
    } catch (error) {
      setState({
        status: 'awaiting_terms',
        registrationToken,
        errorMessage: toDisplayError(
          error,
          'РќРµ СѓРґР°Р»РѕСЃСЊ Р·Р°РІРµСЂС€РёС‚СЊ СЂРµРіРёСЃС‚СЂР°С†РёСЋ.',
        ),
      })
    }
  }, [options.registrationToken, state])

  return useMemo(
    () => ({
      state,
      retry,
      acceptTerms,
    }),
    [acceptTerms, retry, state],
  )
}
