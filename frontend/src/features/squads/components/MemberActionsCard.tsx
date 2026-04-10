import { useState } from 'react'
import toast from 'react-hot-toast'
import { leaveSquad } from '../../../api/squads'
import { toDisplayError } from '../../../api/http'
import AppPortal from '../../../shared/ui/portal/AppPortal'

type Props = {
  authToken: string
  squadId: string
  onChanged: () => Promise<void>
}

export default function MemberActionsCard({ authToken, squadId, onChanged }: Props) {
  const [isLeaving, setIsLeaving] = useState(false)
  const [isLeaveModalOpen, setIsLeaveModalOpen] = useState(false)

  const onLeave = async () => {
    setIsLeaving(true)
    const request = leaveSquad(authToken, squadId)
    toast.promise(request, {
      loading: 'Выходим из сквада...',
      success: 'Вы покинули сквад.',
      error: (cause) => toDisplayError(cause, 'Не удалось покинуть сквад.'),
    })

    try {
      await request
      setIsLeaveModalOpen(false)
      await onChanged()
    } finally {
      setIsLeaving(false)
    }
  }

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">Действия</h2>
      </div>
      <div className="profile-actions">
        <button className="btn danger" type="button" disabled={isLeaving} onClick={() => setIsLeaveModalOpen(true)}>
          {isLeaving ? 'Выход...' : 'Покинуть сквад'}
        </button>
      </div>

      {isLeaveModalOpen ? (
        <AppPortal>
          <div className="ui-modal-backdrop" role="presentation" onClick={() => !isLeaving && setIsLeaveModalOpen(false)}>
            <div className="ui-modal" role="dialog" aria-modal="true" aria-labelledby="leave-squad-modal-title" onClick={(event) => event.stopPropagation()}>
              <div className="ui-modal-header">
                <h2 id="leave-squad-modal-title" className="ui-modal-title">
                  Покинуть сквад?
                </h2>
                <button className="ui-modal-close" type="button" aria-label="Закрыть" onClick={() => setIsLeaveModalOpen(false)} disabled={isLeaving}>
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>Вы покинете текущий сквад. Подтвердите действие.</p>
              </div>
              <div className="ui-modal-footer">
                <button className="btn" type="button" onClick={() => setIsLeaveModalOpen(false)} disabled={isLeaving}>
                  Отменить
                </button>
                <button className="btn danger" type="button" onClick={() => void onLeave()} disabled={isLeaving}>
                  {isLeaving ? 'Выход...' : 'Покинуть'}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </section>
  )
}
