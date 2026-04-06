import {useEffect, useState} from 'react'
import {useNavigate} from 'react-router-dom'
import LoadingState from '../components/LoadingState'
import ErrorState from '../components/ErrorState'
import {type DiscordAuthInitResponse, requestDiscordAuthInit} from '../services/authApi'
import {useAuthStore} from '../store/authStore'
import { tokenManager } from '../services/tokenManager'
import {solvePow} from "../services/pow.ts";
import {useQuery} from "../util/query.ts";

interface InitState {
    status: 'loading' | 'solving_pow' | 'redirecting' | 'error'
    oauthUrl?: string
    errorMessage?: string
    powProgress?: { percents: number, eta: number }
}

export default function AuthStartPage() {
    const [state, setState] = useState<InitState>({status: 'loading'})
    const store = useAuthStore()
    const navigate = useNavigate()
    const query = useQuery()

    useEffect(() => {
        // If we already have a token AND no auth params, redirect to profile
        const existingToken = useAuthStore.getState().token
        const hasAuthParams = query.get('redirectUrl') || query.get('polling')
        
        if (existingToken && tokenManager.validateTokenFormat(existingToken) && !hasAuthParams) {
            navigate('/profile')
            return
        }

        // Only clear data if we're starting a new auth flow (no code from Discord)
        // If code is present, AuthCallbackPage will handle the flow
        if (!query.get('code')) {
            store.setUser(null)
            store.setPoWData(null)
        }
    }, [query, navigate, store.setUser, store.setPoWData])

    useEffect(() => {
        let cancelled = false
        let returnUrl = query.get('redirectUrl')
        let pollingData = query.get('polling')

        async function initAuth() {
            try {
                let data = null as DiscordAuthInitResponse | null;
                if (returnUrl) {
                    data = await requestDiscordAuthInit(returnUrl)
                } else if (pollingData) {
                    data = JSON.parse(atob(pollingData));
                }
                if (!data) {
                    // throw new Error('Return URL or polling data is required')
                    data = await requestDiscordAuthInit("/profile")
                }
                if (cancelled) return

                setState({status: 'solving_pow'})
                const updateProgress = (progress: { percents: number, eta: number }) => {
                    if (cancelled) return
                    setState((prevState) => ({
                        ...prevState,
                        powProgress: progress,
                    }))
                    console.log(progress)
                }
                const powResult = await solvePow(data.powPrefix, data.powComplexity, updateProgress);

                useAuthStore.getState().setPoWData({solution: powResult, prefix: data.powPrefix})
                setState({status: 'redirecting', oauthUrl: data.oauthUrl})
                window.location.href = data.oauthUrl
            } catch (err) {
                if (cancelled) return
                console.error(err)
                const message = err instanceof Error ? err.message : 'Не удалось инициализировать авторизацию'
                setState({status: 'error', errorMessage: message})
            }
        }

        initAuth()
            .catch((err) => {
                if (cancelled) return
                const message = err instanceof Error ? err.message : 'Не удалось инициализировать авторизацию'
                setState({status: 'error', errorMessage: message})
            })

        return () => {
            cancelled = true
        }
    }, [])

    if (state.status === 'loading') {
        return (
            <div className="page auth-start-page">
                <LoadingState title="Подключение к Discord" message="Готовим аутентефикацию…"/>
            </div>
        )
    }

    if (state.status === 'solving_pow') {
        return (
            <div className="page auth-start-page">
                <LoadingState
                    title="Переадресуем на Discord"
                    message="Быстренько убедимся, что вы не бот…"
                >
                    {state.powProgress && (
                        <div className="pow-progress">
                            <progress value={state.powProgress.percents} max={100}/>
                            <p>Осталось примерно: {state.powProgress.eta} секунд</p>
                            {state.powProgress.percents == 100 && <p>Немножко долго получилось, еще чуть чуть...</p>}
                        </div>
                    )}
                </LoadingState>
            </div>
        )
    }

    if (state.status === 'redirecting') {
        return (
            <div className="page auth-start-page">
                <LoadingState
                    title="Переадресуем на Discord"
                    message="Сейчас вы будете перенаправлены на страницу авторизации Discord."
                >
                    {state.oauthUrl && (
                        <button className="btn" type="button" onClick={() => (window.location.href = state.oauthUrl!)}>
                            Если ничего не происходит, нажмите сюда
                        </button>
                    )}
                </LoadingState>
            </div>
        )
    }

    return (
        <div className="page auth-start-page">
            <ErrorState
                title="Ошибка авторизации"
                message={state.errorMessage ?? 'Не удалось инициализировать авторизацию'}
                primaryActionLabel="Попробовать ещё раз"
                onPrimaryAction={() => {
                    setState({status: 'loading'})
                    window.location.reload()
                }}
            />
        </div>
    )
}
