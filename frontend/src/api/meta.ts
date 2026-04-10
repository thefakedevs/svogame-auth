import { request } from './http'

export interface RestrictionMetaResponse {
  key: string
  locale: {
    en: {
      title: string
      description: string
    }
  }
}

export interface SquadConfigResponse {
  maxMembers: number
  inviteTtlHours: number
  nameMinChars: number
  nameMaxChars: number
  nameRegex: string
  imageMaxBytes: number
  imageMaxWidth: number
  imageMaxHeight: number
}

export function getRestrictionMeta(): Promise<RestrictionMetaResponse[]> {
  return request<RestrictionMetaResponse[]>('/api/meta/restrictions')
}

export function getSquadConfig(): Promise<SquadConfigResponse> {
  return request<SquadConfigResponse>('/api/meta/squads/config')
}
