import { request } from './http'

const MINECRAFT_STATUS_URL = 'https://api.mcsrvstat.us/3/svo.svocraft.xyz'
const MINECRAFT_SERVER_ADDRESS = 'svo.svocraft.xyz'

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

export interface MinecraftServerStatusResponse {
  serverAddress: string
  online: boolean
  playersOnline: number
  playersMax: number | null
}

interface McsrvstatResponse {
  online?: boolean
  players?: {
    online?: number
    max?: number
  }
}

export function getRestrictionMeta(): Promise<RestrictionMetaResponse[]> {
  return request<RestrictionMetaResponse[]>('/api/meta/restrictions')
}

export function getSquadConfig(): Promise<SquadConfigResponse> {
  return request<SquadConfigResponse>('/api/meta/squads/config')
}

export async function getMinecraftServerStatus(): Promise<MinecraftServerStatusResponse> {
  const status = await request<McsrvstatResponse>(MINECRAFT_STATUS_URL)

  return {
    serverAddress: MINECRAFT_SERVER_ADDRESS,
    online: status.online ?? false,
    playersOnline: status.players?.online ?? 0,
    playersMax: status.players?.max ?? null,
  }
}
