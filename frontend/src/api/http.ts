export type ApiErrorKind = 'http' | 'network' | 'parse'

export class ApiError extends Error {
  public readonly kind: ApiErrorKind
  public readonly status: number
  public readonly statusText: string
  public readonly details?: unknown

  constructor(params: {
    kind: ApiErrorKind
    message: string
    status?: number
    statusText?: string
    details?: unknown
  }) {
    super(params.message)
    this.name = 'ApiError'
    this.kind = params.kind
    this.status = params.status ?? 0
    this.statusText = params.statusText ?? ''
    this.details = params.details
  }

  isAuthError(): boolean {
    return this.status === 401 || this.status === 403
  }

  isNetworkError(): boolean {
    return this.kind === 'network'
  }

  isServerError(): boolean {
    return this.status >= 500
  }
}

interface RequestOptions extends RequestInit {
  parseAs?: 'json' | 'text'
}

function buildFallbackMessage(status: number, statusText: string): string {
  if (status === 0) {
    return 'Не удалось связаться с сервером. Проверьте подключение и попробуйте еще раз.'
  }
  if (status === 401 || status === 403) {
    return 'Сессия больше недействительна. Войдите снова.'
  }
  if (status === 404) {
    return 'Запрошенные данные не найдены.'
  }
  if (status >= 500) {
    return 'Сервис временно недоступен. Попробуйте позже.'
  }
  return statusText || 'Не удалось выполнить запрос.'
}

async function readResponseBody(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return null
  }

  const contentType = response.headers.get('content-type') ?? ''

  if (contentType.includes('application/json')) {
    try {
      return await response.json()
    } catch {
      return null
    }
  }

  try {
    return await response.text()
  } catch {
    return null
  }
}

function extractErrorMessage(body: unknown): string | null {
  if (!body) return null
  if (typeof body === 'string') {
    const trimmed = body.trim()
    return trimmed || null
  }
  if (typeof body === 'object') {
    const record = body as Record<string, unknown>
    for (const key of ['error', 'message', 'detail']) {
      const value = record[key]
      if (typeof value === 'string' && value.trim()) {
        return value.trim()
      }
    }
  }
  return null
}

export async function request<T>(input: string, options: RequestOptions = {}): Promise<T> {
  const { parseAs = 'json', headers, ...init } = options

  let response: Response
  try {
    response = await fetch(input, {
      ...init,
      headers,
    })
  } catch (error) {
    throw new ApiError({
      kind: 'network',
      message: 'Не удалось связаться с сервером. Проверьте подключение и попробуйте еще раз.',
      details: error,
    })
  }

  const body = await readResponseBody(response)

  if (!response.ok) {
    throw new ApiError({
      kind: 'http',
      status: response.status,
      statusText: response.statusText,
      message: extractErrorMessage(body) ?? buildFallbackMessage(response.status, response.statusText),
      details: body,
    })
  }

  if (parseAs === 'text') {
    return String(body ?? '') as T
  }

  if (body === null) {
    return null as T
  }

  return body as T
}

export async function requestNullable<T>(input: string, options: RequestOptions = {}): Promise<T | null> {
  try {
    return await request<T>(input, options)
  } catch (error) {
    if (error instanceof ApiError && error.status === 404) {
      return null
    }
    throw error
  }
}

export function authHeaders(token: string, headers?: HeadersInit): HeadersInit {
  return {
    Authorization: `Bearer ${token}`,
    ...headers,
  }
}

export function toDisplayError(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    return error.message || buildFallbackMessage(error.status, error.statusText)
  }
  if (error instanceof Error && error.message) {
    return error.message
  }
  return fallback
}
