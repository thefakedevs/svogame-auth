import type { UserProfile } from '../services/authApi'

interface UserProfileCardProps {
  user: UserProfile
}

export default function UserProfileCard({ user }: UserProfileCardProps) {
  return (
    <div className="card user-profile-card">
      <img className="avatar" src={user.avatarUrl} alt={user.username} />
      <div className="user-info">
        <div className="username">{user.username}</div>
        <div className="userid">ID: {user.id}</div>
      </div>
    </div>
  )
}

