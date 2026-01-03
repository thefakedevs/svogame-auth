import React, { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useQuery } from '../util/query'
import { useAuthStore } from '../store/authStore'
import { getCurrentUser, updateNickname, type UserResponse, ApiError } from '../services/userApi'
import { tokenManager } from '../services/tokenManager'
import UserProfileCard from '../components/UserProfileCard'
import NicknameEditor from '../components/NicknameEditor'
import './ProfilePage.css'

interface ProfilePageState {
  status: 'loading' | 'loaded' | 'error' | 'unauthorized'
  user?: UserResponse
  errorMessage?: string
  isUpdatingNickname?: boolean
}

const ProfilePage: React.FC = () => {
  const navigate = useNavigate()
  const query = useQuery()
  const authStore = useAuthStore()
  
  const [state, setState] = useState<ProfilePageState>({
    status: 'loading'
  })

  const fetchUserData = async (token: string) => {
    try {
      const userData = await getCurrentUser(token)
      setState({ 
        status: 'loaded',
        user: userData
      })
    } catch (error) {
      if (error instanceof ApiError && error.isAuthError()) {
        tokenManager.clearToken()
        authStore.setToken(null)
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

    try {
      const updatedUser = await updateNickname(token, newNickname)
      setState(prev => ({ 
        ...prev, 
        user: updatedUser,
        isUpdatingNickname: false
      }))
    } catch (error) {
      setState(prev => ({ ...prev, isUpdatingNickname: false }))
      
      if (error instanceof ApiError && error.isAuthError()) {
        tokenManager.clearToken()
        authStore.setToken(null)
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
          window.history.replaceState(null, '', window.location.pathname + window.location.search.replace(/([?&])token=[^&]*(&|$)/, (m, p1, p2) => p2 ? p1 : ''))
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
    navigate('/auth')
  }

  const renderContent = () => {
    switch (state.status) {
      case 'loading':
        return <div className="loading">Loading profile...</div>
      
      case 'unauthorized':
        return <div className="error">Redirecting to authentication...</div>
      
      case 'error':
        return (
          <div className="error">
            <p>Error: {state.errorMessage}</p>
            <button onClick={() => navigate('/auth')}>
              Return to Authentication
            </button>
          </div>
        )
      
      case 'loaded':
        return (
          <div className="profile-content">
            <h1>Профиль пользователя</h1>
            
            {/* User profile display */}
            <UserProfileCard 
              user={state.user}
            />
            
            {/* Nickname editor */}
            {state.user && (
              <div className="nickname-section">
                <h2>Редактирование никнейма</h2>
                <NicknameEditor
                  currentUsername={state.user.username}
                  onNicknameUpdate={handleNicknameUpdate}
                  isLoading={state.isUpdatingNickname}
                />
                <div className="logout-section">
                  <button className="logout-button" onClick={handleLogout}>Выйти</button>
                </div>
              </div>
            )}
          </div>
        )
      
      default:
        return null
    }
  }

  return (
    <div className="profile-page">
      {renderContent()}
    </div>
  )
}

export default ProfilePage