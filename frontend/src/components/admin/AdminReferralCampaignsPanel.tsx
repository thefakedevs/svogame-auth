import { useCallback, useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import {
  createAdminReferralCampaign,
  getAdminReferralCampaign,
  getAdminReferralCampaignStats,
  listAdminReferralCampaigns,
  patchAdminReferralCampaign,
  revokeAdminReferralCampaign,
  type AdminReferralCampaignResponse,
  type AdminReferralCampaignRewardInput,
  type AdminReferralCampaignStatus,
} from '../../api/admin'
import { toDisplayError } from '../../api/http'
import type { ReferralStatsResponse } from '../../api/referrals'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'

const REFERRAL_PAGE_SIZE = 20

type RewardDraft = {
  assetKey: string
  amount: string
  durationSeconds: string
  metadataText: string
}

type CampaignDraft = {
  code: string
  title: string
  contentCreatorUserId: string
  status: AdminReferralCampaignStatus
  startsAt: string
  endsAt: string
  rewards: RewardDraft[]
}

const emptyRewardDraft = (): RewardDraft => ({
  assetKey: '',
  amount: '',
  durationSeconds: '',
  metadataText: '{}',
})

const emptyCampaignDraft = (): CampaignDraft => ({
  code: '',
  title: '',
  contentCreatorUserId: '',
  status: 'active',
  startsAt: '',
  endsAt: '',
  rewards: [emptyRewardDraft()],
})

function formatDateTime(value?: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('ru-RU')
}

function toDatetimeLocal(value: string | null | undefined): string {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toISOString().slice(0, 16)
}

function fromDatetimeLocal(value: string): string | null {
  if (!value) return null
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? null : date.toISOString()
}

function campaignToDraft(campaign: AdminReferralCampaignResponse): CampaignDraft {
  return {
    code: campaign.code,
    title: campaign.title,
    contentCreatorUserId: campaign.contentCreatorUserId ?? '',
    status: campaign.status,
    startsAt: toDatetimeLocal(campaign.startsAt),
    endsAt: toDatetimeLocal(campaign.endsAt),
    rewards: campaign.rewards.length > 0
      ? campaign.rewards.map((reward) => ({
          assetKey: reward.assetKey,
          amount: reward.amount ? String(reward.amount) : '',
          durationSeconds: reward.durationSeconds ? String(reward.durationSeconds) : '',
          metadataText: JSON.stringify(reward.metadata ?? {}, null, 2),
        }))
      : [emptyRewardDraft()],
  }
}

function normalizeCode(value: string): string {
  return value.trim().toUpperCase()
}

function buildStatsRange() {
  const now = new Date()
  const to = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), 23, 59, 59))
  const from = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() - 6, 0, 0, 0))
  return {
    from: from.toISOString(),
    to: to.toISOString(),
  }
}

function rewardLabel(reward: AdminReferralCampaignResponse['rewards'][number]) {
  const name = reward.assetDisplayName || reward.assetKey
  if (reward.amount && reward.amount > 0) return `${name} x${reward.amount}`
  if (reward.durationSeconds && reward.durationSeconds > 0) return `${name}, ${reward.durationSeconds} сек`
  return name
}

function draftToPayload(draft: CampaignDraft): {
  code: string
  title: string
  contentCreatorUserId: string | null
  status: AdminReferralCampaignStatus
  startsAt: string | null
  endsAt: string | null
  rewards: AdminReferralCampaignRewardInput[]
} {
  const rewards = draft.rewards
    .filter((reward) => reward.assetKey.trim())
    .map((reward) => {
      let metadata: unknown = {}
      if (reward.metadataText.trim()) {
        metadata = JSON.parse(reward.metadataText)
      }

      return {
        assetKey: reward.assetKey.trim(),
        ...(reward.amount.trim() ? { amount: Number(reward.amount) } : {}),
        ...(reward.durationSeconds.trim() ? { durationSeconds: Number(reward.durationSeconds) } : {}),
        metadata,
      }
    })

  return {
    code: normalizeCode(draft.code),
    title: draft.title.trim(),
    contentCreatorUserId: draft.contentCreatorUserId.trim() || null,
    status: draft.status,
    startsAt: fromDatetimeLocal(draft.startsAt),
    endsAt: fromDatetimeLocal(draft.endsAt),
    rewards,
  }
}

function StatusBadge({ status }: { status: AdminReferralCampaignStatus }) {
  const className =
    status === 'active'
      ? 'ui-badge ui-badge-success'
      : status === 'draft'
        ? 'ui-badge ui-badge-neutral'
        : 'ui-badge ui-badge-warning'

  return <span className={className}>{status}</span>
}

function StatsChart({ stats }: { stats: ReferralStatsResponse }) {
  const maxValue = Math.max(1, ...stats.buckets.map((bucket) => bucket.registrations))

  return (
    <div className="admin-referral-chart" aria-label="Дневная статистика регистраций">
      {stats.buckets.map((bucket) => (
        <div key={bucket.date} className="admin-referral-chart__bar-wrap">
          <span className="admin-referral-chart__value">{bucket.registrations}</span>
          <span
            className="admin-referral-chart__bar"
            style={{ height: `${Math.max(8, (bucket.registrations / maxValue) * 100)}%` }}
            aria-hidden
          />
          <span className="admin-referral-chart__date">{bucket.date.slice(5)}</span>
        </div>
      ))}
    </div>
  )
}

export default function AdminReferralCampaignsPanel({ token }: { token: string }) {
  const [campaigns, setCampaigns] = useState<AdminReferralCampaignResponse[]>([])
  const [page, setPage] = useState(1)
  const [totalPages, setTotalPages] = useState(1)
  const [total, setTotal] = useState(0)
  const [isLoadingList, setIsLoadingList] = useState(true)
  const [listError, setListError] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [selectedCampaign, setSelectedCampaign] = useState<AdminReferralCampaignResponse | null>(null)
  const [stats, setStats] = useState<ReferralStatsResponse | null>(null)
  const [detailError, setDetailError] = useState('')
  const [draft, setDraft] = useState<CampaignDraft>(() => emptyCampaignDraft())
  const [isSaving, setIsSaving] = useState(false)

  const statsRange = useMemo(() => buildStatsRange(), [])

  const reloadList = useCallback(async (preferredSelectedId?: string) => {
    setIsLoadingList(true)
    setListError('')
    try {
      const response = await listAdminReferralCampaigns(token, { page, perPage: REFERRAL_PAGE_SIZE })
      setCampaigns(response.items)
      setTotal(response.total)
      setTotalPages(Math.max(1, response.totalPages))
      if (preferredSelectedId) {
        setSelectedId(preferredSelectedId)
      } else if (!selectedId && response.items[0]) {
        setSelectedId(response.items[0].id)
      }
    } catch (cause) {
      setListError(toDisplayError(cause, 'Не удалось загрузить реферальные кампании.'))
    } finally {
      setIsLoadingList(false)
    }
  }, [page, selectedId, token])

  useEffect(() => {
    void reloadList()
  }, [reloadList])

  useEffect(() => {
    if (!selectedId) {
      setSelectedCampaign(null)
      setStats(null)
      setDraft(emptyCampaignDraft())
      return
    }

    let cancelled = false
    setDetailError('')
    setSelectedCampaign(null)
    setStats(null)

    const run = async () => {
      try {
        const [campaign, campaignStats] = await Promise.all([
          getAdminReferralCampaign(token, selectedId),
          getAdminReferralCampaignStats(token, selectedId, statsRange),
        ])
        if (cancelled) return
        setSelectedCampaign(campaign)
        setDraft(campaignToDraft(campaign))
        setStats(campaignStats)
      } catch (cause) {
        if (!cancelled) setDetailError(toDisplayError(cause, 'Не удалось загрузить кампанию.'))
      }
    }

    void run()
    return () => {
      cancelled = true
    }
  }, [selectedId, statsRange, token])

  const createCampaign = async () => {
    if (isSaving) return
    setIsSaving(true)
    try {
      const payload = draftToPayload(draft)
      if (!payload.code || !payload.title) {
        toast.error('Укажите код и название кампании.')
        return
      }
      const created = await createAdminReferralCampaign(token, payload)
      toast.success('Реферальная кампания создана.')
      setDraft(campaignToDraft(created))
      await reloadList(created.id)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось создать кампанию.'))
    } finally {
      setIsSaving(false)
    }
  }

  const saveCampaign = async () => {
    if (!selectedCampaign || isSaving) return
    setIsSaving(true)
    try {
      const payload = draftToPayload(draft)
      if (!payload.title) {
        toast.error('Укажите название кампании.')
        return
      }
      const updated = await patchAdminReferralCampaign(token, selectedCampaign.id, {
        title: payload.title,
        contentCreatorUserId: payload.contentCreatorUserId,
        status: payload.status,
        startsAt: payload.startsAt,
        endsAt: payload.endsAt,
        rewards: payload.rewards,
      })
      setSelectedCampaign(updated)
      setDraft(campaignToDraft(updated))
      toast.success('Кампания обновлена.')
      await reloadList(updated.id)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось сохранить кампанию.'))
    } finally {
      setIsSaving(false)
    }
  }

  const revokeCampaign = async () => {
    if (!selectedCampaign || isSaving) return
    const approved = window.confirm(`Отозвать кампанию "${selectedCampaign.title}"?`)
    if (!approved) return

    setIsSaving(true)
    try {
      const revoked = await revokeAdminReferralCampaign(token, selectedCampaign.id)
      setSelectedCampaign(revoked)
      setDraft(campaignToDraft(revoked))
      toast.success('Кампания отозвана.')
      await reloadList(revoked.id)
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось отозвать кампанию.'))
    } finally {
      setIsSaving(false)
    }
  }

  const updateReward = (index: number, patch: Partial<RewardDraft>) => {
    setDraft((current) => ({
      ...current,
      rewards: current.rewards.map((reward, rewardIndex) => (
        rewardIndex === index ? { ...reward, ...patch } : reward
      )),
    }))
  }

  return (
    <section className="admin-referrals-layout">
      <section className="card admin-card">
        <div className="admin-section-head">
          <div>
            <h2 className="card-title">Реферальные кампании</h2>
            <p className="card-text">Создание, статусы, welcome pack и статистика регистраций.</p>
          </div>
          <span className="ui-badge ui-badge-neutral">Всего {total}</span>
        </div>

        {listError ? <ErrorState message={listError} /> : null}
        {isLoadingList ? <LoadingState title="Загружаем кампании" /> : null}
        {!isLoadingList ? (
          <div className="admin-referral-list">
            {campaigns.map((campaign) => (
              <button
                key={campaign.id}
                type="button"
                className={`admin-row admin-referral-row ${selectedId === campaign.id ? 'is-active' : ''}`}
                onClick={() => setSelectedId(campaign.id)}
              >
                <span className="admin-row-user-text">
                  <strong>{campaign.title}</strong>
                  <small>{campaign.code}</small>
                </span>
                <span className="admin-row-badges">
                  <StatusBadge status={campaign.status} />
                </span>
              </button>
            ))}
            {campaigns.length === 0 ? <p className="admin-inline-muted">Кампаний пока нет.</p> : null}
          </div>
        ) : null}

        <nav className="admin-pagination" aria-label="Нумерация страниц реферальных кампаний">
          <ul className="ui-pagination">
            <li>
              <button type="button" className="ui-pagination-btn" disabled={page <= 1} onClick={() => setPage((value) => Math.max(1, value - 1))}>‹</button>
            </li>
            <li><span className="ui-pagination-ellipsis">{page} / {totalPages}</span></li>
            <li>
              <button type="button" className="ui-pagination-btn" disabled={page >= totalPages} onClick={() => setPage((value) => Math.min(totalPages, value + 1))}>›</button>
            </li>
          </ul>
        </nav>
      </section>

      <section className="card admin-card admin-referral-detail">
        <div className="admin-section-head">
          <div>
            <h2 className="card-title">{selectedCampaign ? 'Редактирование кампании' : 'Новая кампания'}</h2>
            <p className="card-text">Код нормализуется в uppercase. Пустой список наград допустим.</p>
          </div>
          {selectedCampaign ? <StatusBadge status={selectedCampaign.status} /> : null}
        </div>

        {detailError ? <ErrorState message={detailError} /> : null}
        {selectedId && !selectedCampaign && !detailError ? <LoadingState title="Загружаем кампанию" /> : null}

        <div className="admin-shop-form-grid admin-spaced-form">
          <label className="admin-shop-field">
            <span>Код</span>
            <input
              className="ui-input"
              value={draft.code}
              onChange={(event) => setDraft((current) => ({ ...current, code: normalizeCode(event.target.value) }))}
              disabled={Boolean(selectedCampaign)}
              placeholder="CREATOR-ONE"
            />
          </label>
          <label className="admin-shop-field">
            <span>Название</span>
            <input className="ui-input" value={draft.title} onChange={(event) => setDraft((current) => ({ ...current, title: event.target.value }))} />
          </label>
          <label className="admin-shop-field">
            <span>Контентмейкер UUID</span>
            <input className="ui-input" value={draft.contentCreatorUserId} onChange={(event) => setDraft((current) => ({ ...current, contentCreatorUserId: event.target.value }))} placeholder="Опционально" />
          </label>
          <label className="admin-shop-field">
            <span>Статус</span>
            <select className="ui-input" value={draft.status} onChange={(event) => setDraft((current) => ({ ...current, status: event.target.value as AdminReferralCampaignStatus }))}>
              <option value="active">active</option>
              <option value="draft">draft</option>
              <option value="revoked">revoked</option>
            </select>
          </label>
          <label className="admin-shop-field">
            <span>Начало</span>
            <input className="ui-input" type="datetime-local" value={draft.startsAt} onChange={(event) => setDraft((current) => ({ ...current, startsAt: event.target.value }))} />
          </label>
          <label className="admin-shop-field">
            <span>Конец</span>
            <input className="ui-input" type="datetime-local" value={draft.endsAt} onChange={(event) => setDraft((current) => ({ ...current, endsAt: event.target.value }))} />
          </label>
        </div>

        <section className="admin-card-subsection">
          <div className="admin-section-head">
            <div>
              <h3 className="card-title">Награды</h3>
              <p className="card-text">Оставьте поля amount/duration пустыми, если они не нужны для типа ассета.</p>
            </div>
            <button type="button" className="btn btn-sm" onClick={() => setDraft((current) => ({ ...current, rewards: [...current.rewards, emptyRewardDraft()] }))}>
              Добавить награду
            </button>
          </div>

          <div className="admin-referral-rewards">
            {draft.rewards.map((reward, index) => (
              <article key={index} className="admin-referral-reward">
                <label className="admin-shop-field">
                  <span>assetKey</span>
                  <input className="ui-input" value={reward.assetKey} onChange={(event) => updateReward(index, { assetKey: event.target.value })} placeholder="coin_default" />
                </label>
                <label className="admin-shop-field">
                  <span>amount</span>
                  <input className="ui-input" type="number" min="1" value={reward.amount} onChange={(event) => updateReward(index, { amount: event.target.value })} />
                </label>
                <label className="admin-shop-field">
                  <span>durationSeconds</span>
                  <input className="ui-input" type="number" min="1" value={reward.durationSeconds} onChange={(event) => updateReward(index, { durationSeconds: event.target.value })} />
                </label>
                <label className="admin-shop-field admin-shop-field--full">
                  <span>metadata JSON</span>
                  <textarea className="ui-input admin-shop-textarea" value={reward.metadataText} onChange={(event) => updateReward(index, { metadataText: event.target.value })} spellCheck={false} />
                </label>
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => setDraft((current) => ({ ...current, rewards: current.rewards.filter((_, rewardIndex) => rewardIndex !== index) }))}
                >
                  Удалить
                </button>
              </article>
            ))}
          </div>
        </section>

        <div className="admin-shop-actions">
          <button type="button" className="btn" onClick={() => {
            setSelectedId(null)
            setSelectedCampaign(null)
            setStats(null)
            setDraft(emptyCampaignDraft())
          }}>
            Новая кампания
          </button>
          {selectedCampaign ? (
            <>
              <button type="button" className="btn primary" disabled={isSaving} onClick={() => void saveCampaign()}>
                {isSaving ? 'Сохраняем...' : 'Сохранить'}
              </button>
              <button type="button" className="btn danger" disabled={isSaving || selectedCampaign.status === 'revoked'} onClick={() => void revokeCampaign()}>
                Отозвать
              </button>
            </>
          ) : (
            <button type="button" className="btn primary" disabled={isSaving} onClick={() => void createCampaign()}>
              {isSaving ? 'Создаем...' : 'Создать'}
            </button>
          )}
        </div>

        {selectedCampaign ? (
          <section className="admin-card-subsection">
            <div className="admin-metric-strip admin-metric-strip--compact">
              <div className="admin-metric">
                <span>Регистрации</span>
                <strong>{stats?.total ?? 0}</strong>
              </div>
              <div className="admin-metric">
                <span>Создана</span>
                <strong>{formatDateTime(selectedCampaign.createdAt)}</strong>
              </div>
              <div className="admin-metric">
                <span>Награды</span>
                <strong>{selectedCampaign.rewards.length}</strong>
              </div>
            </div>

            {stats ? <StatsChart stats={stats} /> : <LoadingState title="Загружаем статистику" />}

            <div className="admin-referral-current-rewards">
              {selectedCampaign.rewards.map((reward) => (
                <span key={`${reward.assetKey}-${reward.amount ?? 'once'}-${reward.durationSeconds ?? 'permanent'}`} className="ui-badge ui-badge-neutral">
                  {rewardLabel(reward)}
                </span>
              ))}
              {selectedCampaign.rewards.length === 0 ? <p className="admin-inline-muted">Награды не заданы.</p> : null}
            </div>
          </section>
        ) : null}
      </section>
    </section>
  )
}
