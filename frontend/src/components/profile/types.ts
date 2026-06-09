import type {
  SquadInviteResponse,
  SquadMemberResponse,
  SquadResponse,
} from '../../api/squads'
import type { SquadConfigResponse } from '../../api/meta'
import type { UserResponse } from '../../api/users'

export type ProfileTab = 'overview' | 'squads' | 'referrals' | 'settings'
export type ProfileStatus = 'loading' | 'loaded' | 'error' | 'unauthorized'

export interface ProfileDashboardData {
  user: UserResponse
  squad: SquadResponse | null
  squadMembers: SquadMemberResponse[]
  squadInvites: SquadInviteResponse[]
  squadConfig: SquadConfigResponse
}
