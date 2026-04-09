import type {
  SquadConfigResponse,
  SquadInviteResponse,
  SquadMemberResponse,
  SquadResponse,
} from '../../api/profile'
import type { UserResponse } from '../../api/users'

export type ProfileTab = 'overview' | 'squads' | 'settings'
export type ProfileStatus = 'loading' | 'loaded' | 'error' | 'unauthorized'

export interface ProfileDashboardData {
  user: UserResponse
  squad: SquadResponse | null
  squadMembers: SquadMemberResponse[]
  squadInvites: SquadInviteResponse[]
  squadConfig: SquadConfigResponse
}
