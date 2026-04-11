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

interface AuthResponseBase {
  status: 'authorized' | 'terms_required'
}

export interface AuthorizedAuthResponse extends AuthResponseBase {
  status: 'authorized'
  accessToken: string
  id: string
  username: string
  avatarUrl: string
  deliveryMethod: 'redirect' | 'polling'
  deliveryTarget: string
  user: UserProfile
}

export interface TermsRequiredAuthResponse extends AuthResponseBase {
  status: 'terms_required'
  registrationToken: string
}

type RawAuthorizedAuthResponse = {
  status: 'authorized'
  accessToken: string
  id: string
  username: string
  avatarUrl: string
  isSuperuser?: boolean
  deliveryMethod: 'redirect' | 'polling'
  deliveryTarget: string
}

type RawTermsRequiredAuthResponse = {
  status: 'terms_required'
  registrationToken: string
}

export type AuthorizationCallbackResponse = AuthorizedAuthResponse | TermsRequiredAuthResponse

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
    throw new Error('РќРµ РЅР°Р№РґРµРЅ РїР°СЂР°РјРµС‚СЂ code РІ URL.')
  }

  if (!pow) {
    throw new Error('Р›РѕРєР°Р»СЊРЅР°СЏ СЃРµСЃСЃРёСЏ Р°РІС‚РѕСЂРёР·Р°С†РёРё СѓСЃС‚Р°СЂРµР»Р°. РќР°С‡РЅРёС‚Рµ РІС…РѕРґ Р·Р°РЅРѕРІРѕ.')
  }

  const data = await request<RawAuthorizedAuthResponse | RawTermsRequiredAuthResponse>('/api/auth/authorize', {
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

  return normalizeAuthResponse(data)
}

export async function completeRegistration(registrationToken: string): Promise<AuthorizedAuthResponse> {
  const data = await request<RawAuthorizedAuthResponse>('/api/auth/register', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      registrationToken,
      acceptedUserAgreement: true,
      acceptedPrivacyPolicy: true,
    }),
  })

  return normalizeAuthorizedResponse(data)
}

function normalizeAuthResponse(
  data: RawAuthorizedAuthResponse | RawTermsRequiredAuthResponse,
): AuthorizationCallbackResponse {
  if (data.status === 'terms_required') {
    return data
  }

  return normalizeAuthorizedResponse(data)
}

function normalizeAuthorizedResponse(data: RawAuthorizedAuthResponse): AuthorizedAuthResponse {
  return {
    ...data,
    user: {
      id: data.id,
      username: data.username,
      avatarUrl: data.avatarUrl,
      isSuperuser: data.isSuperuser,
    },
  }
}
