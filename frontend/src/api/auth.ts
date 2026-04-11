import { request } from './http'

export interface DiscordAuthInitResponse {
  oauthUrl: string
  powPrefix: string
  powComplexity: number
  deliveryMethod: 'redirect' | 'polling'
}

export interface UserProfile {
  id: string
  username: string
  avatarUrl: string
  isSuperuser?: boolean
}

export interface AuthorizationCallbackResponse {
  accessToken: string
  id: string
  username: string
  avatarUrl: string
  deliveryMethod: 'redirect' | 'polling'
  deliveryTarget: string
  user: UserProfile
}

export async function requestDiscordAuthInit(
  redirectUrl: string | null,
): Promise<DiscordAuthInitResponse> {
  if (!redirectUrl) {
    throw new Error('redirectUrl is required')
  }

  const data = await request<DiscordAuthInitResponse>('/api/auth/prepare', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      redirectUrl,
      deliveryMethod: 'redirect',
    }),
  })

  return {
    oauthUrl: data.oauthUrl,
    powPrefix: data.powPrefix,
    powComplexity: data.powComplexity,
    deliveryMethod: data.deliveryMethod ?? 'redirect',
  }
}

export async function fetchAuthorize(
  code: string,
  pow: { solution: string; prefix: string } | null,
): Promise<AuthorizationCallbackResponse> {
  if (!code) {
    throw new Error('Не найден параметр code в URL.')
  }

  if (!pow) {
    throw new Error('Локальная сессия авторизации устарела. Начните вход заново.')
  }

  const data = await request<
    Omit<AuthorizationCallbackResponse, 'user'> & {
      user?: UserProfile
      isSuperuser?: boolean
    }
  >('/api/auth/authorize', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      discordCode: code,
      powSolution: pow.solution,
      powPrefix: pow.prefix,
    }),
  })

  const responseUser = data.user

  return {
    ...data,
    user: {
      id: responseUser?.id ?? data.id,
      username: responseUser?.username ?? data.username,
      avatarUrl: responseUser?.avatarUrl ?? data.avatarUrl,
      isSuperuser: responseUser?.isSuperuser ?? data.isSuperuser,
    },
  }
}
