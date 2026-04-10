import { getSquadConfig } from '../../../api/meta'
import {
  getMySquad,
  getMySquadInvites,
  getSquadMembers,
  type SquadConfigResponse,
  type SquadInviteResponse,
  type SquadMemberResponse,
  type SquadResponse,
} from '../../../api/squads'
import { getCurrentUser, type UserResponse } from '../../../api/users'
import type { UserProfile } from '../../../api/auth'
import type { ProfileDashboardData } from '../../../components/profile/types'

export async function fetchProfileDashboard(token: string): Promise<ProfileDashboardData> {
  const [user, squad, squadInvites, squadConfig] = await Promise.all([
    getCurrentUser(token),
    getMySquad(token),
    getMySquadInvites(token),
    getSquadConfig(),
  ])

  const squadMembers = squad ? await getSquadMembers(token, squad.id) : []

  return {
    user,
    squad,
    squadMembers,
    squadInvites,
    squadConfig,
  }
}

export async function fetchSquadDashboardSlice(token: string): Promise<{
  squad: SquadResponse | null
  squadMembers: SquadMemberResponse[]
  squadInvites: SquadInviteResponse[]
  squadConfig: SquadConfigResponse
}> {
  const [squad, squadInvites, squadConfig] = await Promise.all([
    getMySquad(token),
    getMySquadInvites(token),
    getSquadConfig(),
  ])

  const squadMembers = squad ? await getSquadMembers(token, squad.id) : []

  return {
    squad,
    squadMembers,
    squadInvites,
    squadConfig,
  }
}

export function toAuthUser(user: UserResponse): UserProfile {
  return {
    id: user.id,
    username: user.username,
    avatarUrl: user.avatarUrl ?? '',
    isSuperuser: user.isSuperuser,
  }
}
