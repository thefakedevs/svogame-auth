import { useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { kickSquadMember } from '../../../api/squads'
import { toDisplayError } from '../../../api/http'
import AppPortal from '../../../shared/ui/portal/AppPortal'
import { initials } from '../lib/format'
import type { SquadSectionProps } from '../types'

type Props = SquadSectionProps & {
  isLeader: boolean
}

export default function SquadMembersCard({ data, isLeader, authToken, onChanged }: Props) {
  const [processingMemberId, setProcessingMemberId] = useState<string | null>(null)
  const [memberToKick, setMemberToKick] = useState<{ id: string; username: string } | null>(null)
  const orderedMembers = useMemo(() => {
    const leader = data.squadMembers.find((member) => member.isLeader)
    if (!leader) return data.squadMembers
    return [leader, ...data.squadMembers.filter((member) => member.id !== leader.id)]
  }, [data.squadMembers])

  const onKick = async (userId: string) => {
    if (!data.squad) return

    setProcessingMemberId(userId)
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
      setProcessingMemberId(null)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Участники</h2>
        <span className="ui-badge ui-badge-neutral">
          {data.squadMembers.length} / {data.squad?.maxMembers ?? data.squadConfig.maxMembers}
        </span>
      </div>
      <div className="profile-member-list">
        {orderedMembers.map((member) => (
          <div key={member.id} className="profile-member">
            <div className="profile-member-avatar">
              {member.avatarUrl ? <img src={member.avatarUrl} alt={member.username} /> : <span>{initials(member.username)}</span>}
            </div>
            <div className="profile-member-copy">
              <strong>{member.username}</strong>
              <span className="profile-subtle">{member.isLeader ? 'Лидер' : 'Участник'}</span>
            </div>
            {isLeader && !member.isLeader ? (
              <button
                className="btn btn-sm"
                type="button"
                disabled={processingMemberId === member.id}
                onClick={() => setMemberToKick({ id: member.id, username: member.username })}
              >
                {processingMemberId === member.id ? 'Исключение...' : 'Кикнуть'}
              </button>
            ) : null}
          </div>
        ))}
      </div>

      {isLeader && memberToKick ? (
        <AppPortal>
          <div className="ui-modal-backdrop" role="presentation" onClick={() => !processingMemberId && setMemberToKick(null)}>
            <div className="ui-modal" role="dialog" aria-modal="true" aria-labelledby="kick-member-modal-title" onClick={(event) => event.stopPropagation()}>
              <div className="ui-modal-header">
                <h2 id="kick-member-modal-title" className="ui-modal-title">
                  Исключить участника?
                </h2>
                <button className="ui-modal-close" type="button" aria-label="Закрыть" onClick={() => setMemberToKick(null)} disabled={Boolean(processingMemberId)}>
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>
                  Участник <strong>{memberToKick.username}</strong> будет исключен из сквада. Подтвердите действие.
                </p>
              </div>
              <div className="ui-modal-footer">
                <button className="btn" type="button" onClick={() => setMemberToKick(null)} disabled={Boolean(processingMemberId)}>
                  Отменить
                </button>
                <button className="btn danger" type="button" onClick={() => void onKick(memberToKick.id)} disabled={Boolean(processingMemberId)}>
                  {processingMemberId ? 'Исключение...' : 'Кикнуть'}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </section>
  )
}
