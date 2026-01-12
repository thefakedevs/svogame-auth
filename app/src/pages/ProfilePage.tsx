import React, { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import toast, { Toaster } from 'react-hot-toast'
import { useQuery } from '../util/query'
import { useAuthStore } from '../store/authStore'
import { getCurrentUser, updateNickname, type UserResponse, ApiError } from '../services/userApi'
import type { UserProfile } from '../services/authApi'
import { tokenManager } from '../services/tokenManager'
import SkinUploadInline from '../components/SkinUploadInline'
import SkinViewer3D from '../components/SkinViewer3D'
import './ProfilePage.css'

interface ProfilePageState {
  status: 'loading' | 'loaded' | 'error' | 'unauthorized'
  user?: UserResponse | UserProfile
  errorMessage?: string
  isUpdatingNickname?: boolean
}

const ProfilePage: React.FC = () => {
  const navigate = useNavigate()
  const query = useQuery()
  const authStore = useAuthStore()
  const [isEditing, setIsEditing] = useState(false)
  const [editNickname, setEditNickname] = useState('')
  const [viewMode, setViewMode] = useState<'avatar' | 'skin'>('avatar')
  
  const [state, setState] = useState<ProfilePageState>({
    status: 'loading'
  })

  const fetchUserData = async (token: string) => {
    try {
      const userData = await getCurrentUser(token)
      const profile: UserProfile = {
        id: userData.id,
        username: userData.username,
        avatarUrl: userData.avatarUrl ?? ''
      }
      authStore.setUser(profile)
      setState({ 
        status: 'loaded',
        user: userData
      })
    } catch (error) {
      if (error instanceof ApiError && error.isAuthError()) {
        tokenManager.clearToken()
        authStore.setToken(null)
        authStore.setUser(null)
        setState({ status: 'unauthorized' })
        navigate('/auth')
      } else {
        setState({ 
          status: 'error', 
          errorMessage: error instanceof Error ? error.message : 'Failed to fetch user data'
        })
      }
    }
  }

  const handleNicknameUpdate = async (newNickname: string) => {
    const token = authStore.token
    if (!token) {
      throw new Error('No authentication token available')
    }

    setState(prev => ({ ...prev, isUpdatingNickname: true }))

    const updatePromise = (async () => {
      const updatedUser = await updateNickname(token, newNickname)
      return updatedUser
    })()

    toast.promise(
      updatePromise,
      {
        loading: 'Обновление никнейма...',
        success: 'Никнейм успешно обновлен!',
        error: (err) => err.message || 'Ошибка при обновлении никнейма'
      }
    )

    try {
      const updatedUser = await updatePromise
      const profile: UserProfile = {
        id: updatedUser.id,
        username: updatedUser.username,
        avatarUrl: updatedUser.avatarUrl ?? ''
      }
      authStore.setUser(profile)
      setState(prev => ({ 
        ...prev, 
        user: updatedUser,
        isUpdatingNickname: false
      }))
      setIsEditing(false)
    } catch (error) {
      setState(prev => ({ ...prev, isUpdatingNickname: false }))
      
      if (error instanceof ApiError && error.isAuthError()) {
        tokenManager.clearToken()
        authStore.setToken(null)
        authStore.setUser(null)
        setState({ status: 'unauthorized' })
        navigate('/auth')
        throw error
      } else {
        const errorMessage = error instanceof Error ? error.message : 'Failed to update nickname'
        throw new Error(errorMessage)
      }
    }
  }

  useEffect(() => {
    const run = async () => {
      const tokenFromUrl = query.get('token')

      if (!tokenFromUrl) {
        const storedToken = authStore.token
        if (!storedToken) {
          setState({ status: 'unauthorized' })
          navigate('/auth')
          return
        }

        // If we already have user data in store, use it directly
        if (authStore.user) {
          setState({ 
            status: 'loaded',
            user: authStore.user
          })
          return
        }

        if (state.status === 'loaded' && state.user) {
          return
        }

        await fetchUserData(storedToken)
        return
      }

      if (!tokenManager.validateTokenFormat(tokenFromUrl)) {
        setState({
          status: 'error',
          errorMessage: 'Invalid token format'
        })
        return
      }

      try {
        await tokenManager.storeToken(tokenFromUrl)
        authStore.setToken(tokenFromUrl)

        await fetchUserData(tokenFromUrl)

        try {
          const url = new URL(window.location.href)
          url.searchParams.delete('token')
          // Keep other query params if any
          navigate(url.pathname + url.search, { replace: true })
        } catch (err) {
          // fallback: replace history without token
          window.history.replaceState(null, '', window.location.pathname + window.location.search.replace(/([?&])token=[^&]*(&|$)/, (_m, p1, p2) => p2 ? p1 : ''))
        }
      } catch (err) {
        console.error('Error storing token or fetching user data', err)
        setState({
          status: 'error',
          errorMessage: err instanceof Error ? err.message : 'Failed to process authentication token'
        })
      }
    }

    run()
  }, [query, navigate, authStore.token])

  const handleLogout = () => {
    tokenManager.clearToken()
    authStore.setToken(null)
    authStore.setUser(null)
    navigate('/auth')
  }

  const renderContent = () => {
    switch (state.status) {
      case 'loading':
        return <div className="loading">Загрузка профиля...</div>
      
      case 'unauthorized':
        return <div className="error">Перенаправление на аутентификацию...</div>
      
      case 'error':
        return (
          <div className="error">
            <p>Ошибка: {state.errorMessage}</p>
            <button onClick={() => {
              window.localStorage.clear()
              navigate('/auth')}
            }>
              Вернуться к аутентификации
            </button>
          </div>
        )
      
      case 'loaded':
        if (isEditing) {
          return (
            <div className="profile-container">
              <div className="profile-view-side">
                <div className="avatar-circle">
                  <img
                    src={state.user?.avatarUrl ?? ''}
                    alt={state.user?.username}
                  />
                </div>
                <h2 className="username-display">{state.user?.username}</h2>
              </div>

              <div className="profile-edit-side">
                <h2>Редактировать профиль</h2>

                {/* Nickname Editor */}
                <div className="edit-field">
                  <label>Никнейм</label>
                  <input
                    type="text"
                    value={editNickname}
                    onChange={(e) => setEditNickname(e.target.value)}
                    placeholder="Введите никнейм (3-16 символов, a-z, 0-9, _)"
                    maxLength={16}
                    disabled={state.isUpdatingNickname}
                  />
                  <div className="validation-hint">
                    3-16 символов, только буквы, цифры и _
                  </div>
                </div>

                {/* Skin Upload */}
                <div className="edit-field">
                  <label>Загрузка скина</label>
                  <div className="skin-upload-compact">
                    <SkinUploadInline userUuid={state.user?.id ?? ''} />
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="edit-actions">
                  <button
                    className="btn-save"
                    onClick={() => {
                      if (editNickname && editNickname !== state.user?.username) {
                        handleNicknameUpdate(editNickname)
                      } else {
                        setIsEditing(false)
                      }
                    }}
                    disabled={state.isUpdatingNickname}
                  >
                    {state.isUpdatingNickname ? 'Сохранение...' : 'Сохранить'}
                  </button>
                  <button
                    className="btn-cancel"
                    onClick={() => {
                      setIsEditing(false)
                      setEditNickname(state.user?.username ?? '')
                    }}
                    disabled={state.isUpdatingNickname}
                  >
                    Отменить
                  </button>
                </div>
              </div>
            </div>
          )
        }

        return (
          <div className="profile-container">
            <div className="profile-view-side">
              <div className="view-mode-toggle">
                <button
                  className={`view-mode-btn ${viewMode === 'avatar' ? 'active' : ''}`}
                  onClick={() => setViewMode('avatar')}
                >
                  Аватар
                </button>
                <button
                  className={`view-mode-btn ${viewMode === 'skin' ? 'active' : ''}`}
                  onClick={() => setViewMode('skin')}
                >
                  3D Скин
                </button>
              </div>

              {viewMode === 'avatar' ? (
                <div className="avatar-circle">
                  <img
                    src={state.user?.avatarUrl ?? ''}
                    alt={state.user?.username}
                  />
                </div>
              ) : (
                <div className="skin-viewer-container">
                  <SkinViewer3D
                    skinUrl={`https://skins.launcher.artembay.ru/skin/${state.user?.id}`}
                    width={300}
                    height={350}
                  />
                </div>
              )}

              <h2 className="username-display">{state.user?.username}</h2>
            </div>

            <div className="profile-info-side">
              <button
                className="btn-edit-profile"
                onClick={() => {
                  setIsEditing(true)
                  setEditNickname(state.user?.username ?? '')
                }}
              >
                Редактировать профиль
              </button>
              <button
                className="btn-logout"
                onClick={handleLogout}
              >
                Выйти из аккаунта
              </button>
            </div>
          </div>
        )
      
      default:
        return null
    }
  }

  return (
    <div className="profile-page">
      <Toaster
        position="top-right"
        toastOptions={{
          duration: 3000,
          style: {
            background: '#1e1e2e',
            color: '#fff',
            border: '1px solid rgba(88, 101, 242, 0.3)',
            borderRadius: '12px',
          },
          success: {
            iconTheme: {
              primary: '#3ba55c',
              secondary: '#fff',
            },
          },
          error: {
            iconTheme: {
              primary: '#ed4245',
              secondary: '#fff',
            },
          },
        }}
      />
      {renderContent()}
    </div>
  )
}

export default ProfilePage