import { useAuthStore } from '../store/authStore'
import { validateTokenFormat } from '../services/tokenManager'

export function getAuthToken(): string | null {
    return useAuthStore.getState().token
}

export function setAuthToken(token: string): void {
    useAuthStore.getState().setToken(token)
}

export function removeAuthToken(): void {
    useAuthStore.getState().setToken(null)
    useAuthStore.getState().setUser(null)
}

export function isAuthenticated(): boolean {
    const token = getAuthToken()
    return token !== null && validateTokenFormat(token)
}
