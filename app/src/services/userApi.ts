/**
 * User API service for profile management operations
 * Implements endpoints according to OpenAPI specification
 */

// TypeScript interfaces matching OpenAPI schemas
export interface UserResponse {
  id: string
  discordId: string
  username: string
  avatarUrl: string | null
  email: string | null
  isActive: boolean
  lastLoginAt: string // ISO date-time
  createdAt: string // ISO date-time
}

export interface UpdateNicknameRequest {
  nickname: string
}

export interface UserAPI {
  getCurrentUser(token: string): Promise<UserResponse>
  updateNickname(token: string, nickname: string): Promise<UserResponse>
}

/**
 * API Error class for handling HTTP errors
 */
export class ApiError extends Error {
  public status: number
  public statusText: string

  constructor(
    status: number,
    statusText: string,
    message?: string
  ) {
    super(message || `${status} ${statusText}`)
    this.name = 'ApiError'
    this.status = status
    this.statusText = statusText
  }

  /**
   * Checks if the error is an authentication error (401/403)
   */
  isAuthError(): boolean {
    return this.status === 401 || this.status === 403
  }
}

/**
 * Handles authentication errors by clearing tokens and redirecting
 * @param error ApiError to handle
 */
function handleAuthError(error: ApiError): void {
  if (error.isAuthError()) {
    // Clear token from auth store
    import('../services/tokenManager').then(({ clearToken }) => {
      clearToken()
    })
    
    // Redirect to auth page
    window.location.href = '/auth'
  }
}

/**
 * Fetches current user information from GET /api/user/me
 * @param token JWT token for authentication
 * @returns Promise<UserResponse> Current user data
 * @throws ApiError for HTTP errors
 */
export async function getCurrentUser(token: string): Promise<UserResponse> {
  try {
    const response = await fetch('/api/user/me', {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    })

    if (!response.ok) {
      let errorMessage = response.statusText;
      try {
        const errorBody = await response.json();
        if (errorBody && errorBody.error) {
          errorMessage = errorBody.error;
        }
      } catch (e) {
        console.log(e);
      }

      const error = new ApiError(response.status, response.statusText, errorMessage)

      // Handle authentication errors
      if (error.isAuthError()) {
        handleAuthError(error)
      }
      
      throw error
    }

    const userData: UserResponse = await response.json()
    return userData
  } catch (error) {
    if (error instanceof ApiError) {
      throw error
    }
    // Handle network errors or other unexpected errors
    throw new ApiError(0, 'Network Error', 'Failed to fetch user data')
  }
}

/**
 * Updates user nickname via POST /api/user/me/nickname
 * @param token JWT token for authentication
 * @param nickname New nickname to set
 * @returns Promise<UserResponse> Updated user data
 * @throws ApiError for HTTP errors
 */
export async function updateNickname(token: string, nickname: string): Promise<UserResponse> {
  try {
    const requestBody: UpdateNicknameRequest = { nickname }

    const response = await fetch('/api/user/me/nickname', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(requestBody)
    })

    if (!response.ok) {
      let errorMessage = response.statusText;
      try {
        const errorBody = await response.json();
        if (errorBody && errorBody.error) {
          errorMessage = errorBody.error;
        }
      } catch (e) {
        // Ignore JSON parsing errors
      }

      const error = new ApiError(response.status, response.statusText, errorMessage)

      // Handle authentication errors
      if (error.isAuthError()) {
        handleAuthError(error)
      }
      
      throw error
    }

    const userData: UserResponse = await response.json()
    return userData
  } catch (error) {
    if (error instanceof ApiError) {
      throw error
    }
    // Handle network errors or other unexpected errors
    throw new ApiError(0, 'Network Error', 'Failed to update nickname')
  }
}

/**
 * UserAPI implementation with all service methods
 */
export const userApi: UserAPI = {
  getCurrentUser,
  updateNickname
}