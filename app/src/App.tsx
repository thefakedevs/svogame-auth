import './App.css'
import { BrowserRouter, Route, Routes, Navigate, useLocation } from 'react-router-dom'
import AuthStartPage from './pages/AuthStartPage.tsx'
import AuthCallbackPage from './pages/AuthCallbackPage.tsx'
import UserEditPage from './pages/UserEditPage.tsx'
import ProfilePage from './pages/ProfilePage.tsx'
import TokenHandler from './pages/TokenHandler.tsx'

function App() {
  return (
    <BrowserRouter>
      <div className="app-root">
        <Routes>
          <Route path="/" element={<Navigate to="/auth" replace />} />
          <Route path="/auth" element={<AuthEntry />} />
          <Route path="/token" element={<TokenHandler />} />
          <Route path="/profile" element={<ProfilePage />} />
          <Route path="/edit-nickname" element={<UserEditPage />} />
          <Route path="*" element={<div className="not-found">Страница не найдена</div>} />
        </Routes>
      </div>
    </BrowserRouter>
  )
}

function AuthEntry() {
  const { search } = useLocation()
  const params = new URLSearchParams(search)
  const code = params.get('code')

  if (code) {
    return <AuthCallbackPage />
  }
  return <AuthStartPage />
}

export default App
