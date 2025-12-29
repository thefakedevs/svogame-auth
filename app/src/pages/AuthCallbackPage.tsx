import {useEffect, useState} from 'react'
import LoadingState from '../components/LoadingState'
import ErrorState from '../components/ErrorState'
import UserProfileCard from '../components/UserProfileCard'
import {type AuthorizationCallbackResponse, fetchAuthorize} from '../services/authApi'
import {useAuthStore} from '../store/authStore'
import {useQuery} from "../util/query.ts";

interface CallbackState {
    status: 'loadingProfile' | 'success' | 'error' | 'delivering'
    user?: AuthorizationCallbackResponse
    errorMessage?: string
}

export default function AuthCallbackPage() {
    const query = useQuery()
    // const navigate = useNavigate()
    const [state, setState] = useState<CallbackState>({status: 'loadingProfile'})
    const {setUser, setPoWData} = useAuthStore()

    useEffect(() => {
        let cancelled = false

        async function loadProfile() {
            try {
                const code = query.get('code') ?? ''
                const pow = useAuthStore.getState().powData

                const data = await fetchAuthorize(code, pow)
                if (cancelled) return

                setPoWData(null)

                if (data.deliveryMethod == 'redirect') {
                    setState({status: 'success', user: data})
                } else if (data.deliveryMethod == 'polling') {
                    setState({status: 'delivering', user: data})
                } else {
                    throw new Error('Неизвестный метод доставки данных: ' + data.deliveryMethod)
                }
            } catch (err) {
                if (cancelled) return
                const message = err instanceof Error ? err.message : 'Ошибка при загрузке профиля'
                setState({status: 'error', errorMessage: message})
            }
        }

        loadProfile()

        return () => {
            cancelled = true
        }
    }, [query, setPoWData, setUser])

    const finishDelivery = function() {
        if (!state.user) return

        if (state.user.deliveryMethod === 'redirect') {
            window.location.href = state.user.deliveryTarget + "?token=" + encodeURIComponent(state.user.accessToken)
        } else if (state.user.deliveryMethod === 'polling') {

        } else {
            console.error('Unknown delivery method:', state.user.deliveryMethod)
        }
    }

    if (state.status === 'loadingProfile') {
        return (
            <div className="page auth-callback-page">
                <LoadingState title="Загружаем профиль Discord" message="Проверяем вашу авторизацию…"/>
            </div>
        )
    }

    if (state.status === 'error') {
        //const isNonceOrCodeIssue = state.errorMessage?.includes('nonce') || state.errorMessage?.includes('code')

        return (
            <div className="page auth-callback-page">
                <ErrorState
                    title="Ошибка авторизации, начните заново"
                    message={state.errorMessage ?? 'Произошла ошибка при авторизации'}
                    //primaryActionLabel={isNonceOrCodeIssue ? 'Начать авторизацию заново' : 'Попробовать ещё раз'}
                    /*onPrimaryAction={() => {
                        navigate('/auth', {replace: true})
                    }}*/
                />
            </div>
        )
    }

    if (state.status === 'success' && state.user) {
        return (
            <div className="page auth-callback-page">
                <UserProfileCard user={state.user}/>
                <div className="card actions-card">
                    <p className="card-text">
                        Нажмите «Войти», чтобы завершить авторизацию.
                    </p>
                    <button className="btn primary" type="button" onClick={finishDelivery}>
                        Войти
                    </button>
                </div>
            </div>
        )
    }

    if (state.status === 'delivering' && state.user) {
        return (
            <div className="page auth-callback-page">
                <UserProfileCard user={state.user}/>
                <div className="card actions-card">
                    <p className="card-text">
                        Авторизация завершается
                    </p>
                    <button className="btn primary" type="button" onClick={finishDelivery}>
                        Войти
                    </button>
                </div>
            </div>
        )
    }

    return null
}
