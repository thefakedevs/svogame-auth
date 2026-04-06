/**
 * TokenManager utility service for handling JWT token extraction and validation
 */

export interface TokenManager {
  extractTokenFromUrl(searchParams: URLSearchParams): string | null
  validateTokenFormat(token: string): boolean
  storeToken(token: string): Promise<void>
  clearToken(): Promise<void>
  getToken(): Promise<string | null>
}

/**
 * Extracts JWT token from URL query parameters
 * @param searchParams URLSearchParams object from the current URL
 * @returns JWT token string or null if not found
 */
export function extractTokenFromUrl(searchParams: URLSearchParams): string | null {
  const token = searchParams.get('token')
  return token && token.trim() !== '' ? token : null
}

/**
 * Validates basic JWT token format (three base64 segments separated by dots)
 * @param token JWT token string to validate
 * @returns true if token has valid JWT structure, false otherwise
 */
export function validateTokenFormat(token: string): boolean {
  if (!token || typeof token !== 'string') {
    return false
  }

  // JWT should have exactly 3 parts separated by dots
  const parts = token.split('.')
  if (parts.length !== 3) {
    return false
  }

  // Each part should be non-empty and contain valid base64url characters
  const base64UrlPattern = /^[A-Za-z0-9_-]+$/
  return parts.every(part => part.length > 0 && base64UrlPattern.test(part))
}

/**
 * Stores JWT token in the auth store
 * @param token JWT token to store
 */
export function storeToken(token: string): Promise<void> {
  // Import dynamically to avoid circular dependencies
  return import('../store/authStore')
    .then(({ useAuthStore }) => {
      useAuthStore.getState().setToken(token)
    })
    .catch(err => {
      // Surface import/set errors to caller
      console.error('tokenManager.storeToken failed', err)
      throw err
    })
}

/**
 * Clears JWT token from the auth store
 */
export function clearToken(): Promise<void> {
  // Import dynamically to avoid circular dependencies
  return import('../store/authStore')
    .then(({ useAuthStore }) => {
      useAuthStore.getState().setToken(null)
    })
    .catch(err => {
      console.error('tokenManager.clearToken failed', err)
      throw err
    })
}

export async function getToken(): Promise<string | null> {
  // Import dynamically to avoid circular dependencies
  const { useAuthStore } = await import('../store/authStore')
  return useAuthStore.getState().token
}

/**
 * TokenManager implementation with all utility functions
 */
export const tokenManager: TokenManager = {
  extractTokenFromUrl,
  validateTokenFormat,
  storeToken,
  clearToken,
  getToken
}