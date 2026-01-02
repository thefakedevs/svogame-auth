const TOKEN_KEY = 'auth-token'

export function getAuthToken(): string | null {
    return localStorage.getItem(TOKEN_KEY)
}

export function setAuthToken(token: string): void {
    localStorage.setItem(TOKEN_KEY, token)
}

export function removeAuthToken(): void {
    localStorage.removeItem(TOKEN_KEY)
}

export function isAuthenticated(): boolean {
    return getAuthToken() !== null
}