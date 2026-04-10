import { useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { kickSquadMember, revokeSquadInvite } from '../../../api/squads'
import { toDisplayError } from '../../../api/http'
import AppPortal from '../../../shared/ui/portal/AppPortal'
import { initials } from '../lib/format'
import type { SquadSectionProps } from '../types'

type Props = SquadSectionProps & {
  isLeader: boolean
}

export default function SquadMembersCard({ data, isLeader, authToken, onChanged }: Props) {
  const [processingActionKey, setProcessingActionKey] = useState<string | null>(null)
  const [memberToKick, setMemberToKick] = useState<{ id: string; username: string } | null>(null)

  const { activeMembers, pendingInviteMembers } = useMemo(() => {
    const pendingMembers = data.squadMembers.filter((member) => member.isPendingInvite)
    const regularMembers = data.squadMembers.filter((member) => !member.isPendingInvite)
    const leader = regularMembers.find((member) => member.isLeader)
    const orderedRegularMembers = leader
      ? [leader, ...regularMembers.filter((member) => member.id !== leader.id)]
      : regularMembers

    return {
      activeMembers: orderedRegularMembers,
      pendingInviteMembers: pendingMembers,
    }
  }, [data.squadMembers])

  const visibleMembers = useMemo(
    () => [...activeMembers, ...pendingInviteMembers],
    [activeMembers, pendingInviteMembers],
  )

  const onKick = async (userId: string) => {
    if (!data.squad) return

    const actionKey = `kick:${userId}`
    setProcessingActionKey(actionKey)
    const request = kickSquadMember(authToken, data.squad.id, userId)
    toast.promise(request, {
      loading: 'Исключаем участника...',
      success: 'Участник исключен из сквада.',
      error: (cause) => toDisplayError(cause, 'Не удалось исключить участника.'),
    })

    try {
      await request
      setMemberToKick(null)
      await onChanged()
    } finally {
      setProcessingActionKey(null)
    }
  }

  const onRevokeInvite = async (inviteId: string) => {
    const actionKey = `invite:${inviteId}`
    setProcessingActionKey(actionKey)
    const request = revokeSquadInvite(authToken, inviteId)
    toast.promise(request, {
      loading: 'Отменяем инвайт...',
      success: 'Инвайт отменен.',
      error: (cause) => toDisplayError(cause, 'Не удалось отменить инвайт.'),
    })

    try {
      await request
      await onChanged()
    } finally {
      setProcessingActionKey(null)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Участники</h2>
        <span className="ui-badge ui-badge-neutral">
          {activeMembers.length} / {data.squad?.maxMembers ?? data.squadConfig.maxMembers}
        </span>
      </div>
      <div className="profile-member-list">
        {visibleMembers.map((member) => {
          const isPendingInvite = member.isPendingInvite
          const pendingActionKey = member.inviteId ? `invite:${member.inviteId}` : null
          const kickActionKey = `kick:${member.id}`
          const isRevokingInvite = pendingActionKey !== null && processingActionKey === pendingActionKey
          const isKickingMember = processingActionKey === kickActionKey

          return (
            <div
              key={member.inviteId ?? member.id}
              className={`profile-member ${isPendingInvite ? 'profile-member--pending' : ''}`}
            >
              <div className="profile-member-avatar">
                {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} /> : <span>{initials(member.username)}</span>}
              </div>
              <div className="profile-member-copy">
                <strong>{member.username}</strong>
                <span className="profile-subtle">
                  {member.isLeader ? 'Лидер' : isPendingInvite ? 'Ожидается' : 'Участник'}
                </span>
              </div>
              {isLeader && isPendingInvite && member.inviteId ? (
                <div className="profile-member-action">
                  <button
                    className="btn btn-sm profile-member-pending-action"
                    type="button"
                    disabled={isRevokingInvite}
                    onClick={() => void onRevokeInvite(member.inviteId!)}
                  >
                    {isRevokingInvite ? 'Отмена...' : 'Отменить инвайт'}
                  </button>
                </div>
              ) : null}
              {isLeader && !isPendingInvite && !member.isLeader ? (
                <button
                  className="btn btn-sm"
                  type="button"
                  disabled={isKickingMember}
                  onClick={() => setMemberToKick({ id: member.id, username: member.username })}
                >
                  {isKickingMember ? 'Исключение...' : 'Кикнуть'}
                </button>
              ) : null}
            </div>
          )
        })}
      </div>

      {isLeader && memberToKick ? (
        <AppPortal>
          <div className="ui-modal-backdrop" role="presentation" onClick={() => !processingActionKey && setMemberToKick(null)}>
            <div className="ui-modal" role="dialog" aria-modal="true" aria-labelledby="kick-member-modal-title" onClick={(event) => event.stopPropagation()}>
              <div className="ui-modal-header">
                <h2 id="kick-member-modal-title" className="ui-modal-title">
                  Исключить участника?
                </h2>
                <button className="ui-modal-close" type="button" aria-label="Закрыть" onClick={() => setMemberToKick(null)} disabled={Boolean(processingActionKey)}>
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>
                  Участник <strong>{memberToKick.username}</strong> будет исключен из сквада. Подтвердите действие.
                </p>
              </div>
              <div className="ui-modal-footer">
                <button className="btn" type="button" onClick={() => setMemberToKick(null)} disabled={Boolean(processingActionKey)}>
                  Отменить
                </button>
                <button className="btn danger" type="button" onClick={() => void onKick(memberToKick.id)} disabled={Boolean(processingActionKey)}>
                  {processingActionKey === `kick:${memberToKick.id}` ? 'Исключение...' : 'Кикнуть'}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </section>
  )
}
