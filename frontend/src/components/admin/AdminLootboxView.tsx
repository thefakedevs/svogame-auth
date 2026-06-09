import { useEffect, useMemo, useState } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../../api/http'
import { listAllAdminAssets, type AssetResponse } from '../../api/inventory'
import {
  createAdminLootboxDrop,
  deleteAdminLootboxDrop,
  getAdminLootbox,
  patchAdminLootbox,
  patchAdminLootboxDrop,
  type LootboxDetailResponse,
  type LootboxDropResponse,
} from '../../api/lootboxes'
import { paths } from '../../routes/paths'
import ErrorState from '../ErrorState'
import LoadingState from '../LoadingState'
import AdminAssetSelect from './AdminAssetSelect'
import AdminLink from './AdminLink'

type DropDraft = {
  amount: string
  duplicateCompensationAmount: string
  durationSeconds: string
  weight: string
  title: string
  isActive: boolean
  sortOrder: string
}

type NewDropDraft = DropDraft & {
  rewardAssetKey: string
}

function metadataToText(value: unknown) {
  if (value == null) return '{}'
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function parseJsonText(value: string) {
  const trimmed = value.trim()
  return trimmed ? JSON.parse(trimmed) as unknown : {}
}

function draftFromDrop(drop: LootboxDropResponse): DropDraft {
  return {
    amount: drop.amount != null ? String(drop.amount) : '',
    duplicateCompensationAmount: drop.duplicateCompensationAmount != null ? String(drop.duplicateCompensationAmount) : '',
    durationSeconds: drop.durationSeconds != null ? String(drop.durationSeconds) : '',
    weight: String(drop.weight),
    title: titleFromI18n(drop.titleI18n),
    isActive: drop.isActive,
    sortOrder: String(drop.sortOrder),
  }
}

function titleFromI18n(value: unknown) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return ''
  const record = value as Record<string, unknown>
  const preferred = record['ru-RU'] ?? record.ru ?? record.en
  if (typeof preferred === 'string') return preferred
  const fallback = Object.values(record).find((item): item is string => typeof item === 'string')
  return fallback ?? ''
}

const emptyNewDrop: NewDropDraft = {
  rewardAssetKey: '',
  amount: '1',
  duplicateCompensationAmount: '',
  durationSeconds: '',
  weight: '1',
  title: '',
  isActive: true,
  sortOrder: '0',
}

function isRewardAsset(asset: AssetResponse) {
  return asset.isActive
    && (asset.ownershipModel === 'stackable' || asset.ownershipModel === 'expirable' || asset.ownershipModel === 'entitlement')
}

export default function AdminLootboxView({ token, lootboxId }: { token: string; lootboxId: string }) {
  const [detail, setDetail] = useState<LootboxDetailResponse | null>(null)
  const [loadError, setLoadError] = useState('')
  const [assets, setAssets] = useState<AssetResponse[]>([])
  const [displayName, setDisplayName] = useState('')
  const [description, setDescription] = useState('')
  const [isPublic, setIsPublic] = useState(false)
  const [isActive, setIsActive] = useState(false)
  const [metadataText, setMetadataText] = useState('{}')
  const [dropDrafts, setDropDrafts] = useState<Record<string, DropDraft>>({})
  const [newDrop, setNewDrop] = useState<NewDropDraft>(emptyNewDrop)
  const [isSavingDefinition, setIsSavingDefinition] = useState(false)
  const [mutatingDropId, setMutatingDropId] = useState<string | null>(null)

  const load = async () => {
    setLoadError('')
    setDetail(null)
    try {
      const [loadedDetail, loadedAssets] = await Promise.all([
        getAdminLootbox(token, lootboxId),
        listAllAdminAssets(token, 100),
      ])
      setDetail(loadedDetail)
      setAssets(loadedAssets.filter(isRewardAsset))
      setDisplayName(loadedDetail.definition.assetDisplayName)
      setDescription(loadedDetail.definition.assetDescription ?? '')
      setIsPublic(loadedDetail.definition.isPublic)
      setIsActive(loadedDetail.definition.isActive)
      setMetadataText(metadataToText(loadedDetail.definition.metadata))
      setDropDrafts(Object.fromEntries(loadedDetail.drops.map((drop) => [drop.id, draftFromDrop(drop)])))
    } catch (cause) {
      setLoadError(toDisplayError(cause, 'Не удалось загрузить лутбокс.'))
    }
  }

  useEffect(() => {
    void load()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, lootboxId])

  const assetsByKey = useMemo(() => new Map(assets.map((asset) => [asset.key, asset])), [assets])

  const saveDefinition = async () => {
    if (!detail || isSavingDefinition) return
    if (!displayName.trim()) {
      toast.error('Укажите название лутбокса.')
      return
    }
    let metadata: unknown
    try {
      metadata = parseJsonText(metadataText)
    } catch {
      toast.error('Metadata должен быть валидным JSON.')
      return
    }

    setIsSavingDefinition(true)
    try {
      const updated = await patchAdminLootbox(token, detail.definition.id, {
        display_name: displayName.trim(),
        description: description.trim() || null,
        is_active: isActive,
        is_public: isPublic,
        metadata,
      })
      setDetail(updated)
      setDisplayName(updated.definition.assetDisplayName)
      setDescription(updated.definition.assetDescription ?? '')
      setIsPublic(updated.definition.isPublic)
      setIsActive(updated.definition.isActive)
      setMetadataText(metadataToText(updated.definition.metadata))
      toast.success('Лутбокс обновлен.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось сохранить лутбокс.'))
    } finally {
      setIsSavingDefinition(false)
    }
  }

  const updateDropDraft = (dropId: string, patch: Partial<DropDraft>) => {
    setDropDrafts((prev) => ({ ...prev, [dropId]: { ...prev[dropId], ...patch } }))
  }

  const buildDropPatch = (draft: DropDraft, assetKey: string) => {
    const asset = assetsByKey.get(assetKey)
    const weight = Number.parseInt(draft.weight, 10)
    const sortOrder = Number.parseInt(draft.sortOrder, 10)
    if (!asset) throw new Error('Reward asset не найден в списке активных ассетов.')
    if (!Number.isFinite(weight) || weight <= 0) throw new Error('weight должен быть больше 0.')
    if (!Number.isFinite(sortOrder)) throw new Error('sortOrder должен быть числом.')

    const title_i18n = draft.title.trim() ? { 'ru-RU': draft.title.trim() } : {}
    const duplicateCompensationAmount = draft.duplicateCompensationAmount.trim()
      ? Number.parseInt(draft.duplicateCompensationAmount, 10)
      : null
    if (duplicateCompensationAmount !== null && (!Number.isFinite(duplicateCompensationAmount) || duplicateCompensationAmount < 0)) {
      throw new Error('duplicateCompensationAmount должен быть числом не меньше 0 или пустым.')
    }

    if (asset.ownershipModel === 'stackable') {
      const amount = Number.parseInt(draft.amount, 10)
      if (!Number.isFinite(amount) || amount <= 0) throw new Error('amount должен быть больше 0 для stackable.')
      return {
        amount,
        duplicate_compensation_amount: duplicateCompensationAmount,
        duration_seconds: null,
        weight,
        title_i18n,
        is_active: draft.isActive,
        sort_order: sortOrder,
      }
    }

    if (asset.ownershipModel === 'entitlement') {
      return {
        amount: null,
        duplicate_compensation_amount: duplicateCompensationAmount,
        duration_seconds: null,
        weight,
        title_i18n,
        is_active: draft.isActive,
        sort_order: sortOrder,
      }
    }

    if (asset.ownershipModel !== 'expirable') throw new Error('API лутбоксов поддерживает только stackable, expirable и entitlement rewards.')
    const durationSeconds = Number.parseInt(draft.durationSeconds, 10)
    if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) {
      throw new Error('durationSeconds должен быть больше 0 для expirable.')
    }
    return {
      amount: null,
      duplicate_compensation_amount: duplicateCompensationAmount,
      duration_seconds: durationSeconds,
      weight,
      title_i18n,
      is_active: draft.isActive,
      sort_order: sortOrder,
    }
  }

  const saveDrop = async (drop: LootboxDropResponse) => {
    if (!detail || mutatingDropId) return
    const draft = dropDrafts[drop.id]
    if (!draft) return

    setMutatingDropId(drop.id)
    try {
      const updated = await patchAdminLootboxDrop(token, detail.definition.id, drop.id, buildDropPatch(draft, drop.rewardAssetKey))
      setDetail(updated)
      setDropDrafts(Object.fromEntries(updated.drops.map((item) => [item.id, draftFromDrop(item)])))
      toast.success('Содержимое лутбокса обновлено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось обновить содержимое.'))
    } finally {
      setMutatingDropId(null)
    }
  }

  const addDrop = async () => {
    if (!detail || mutatingDropId) return
    if (!newDrop.rewardAssetKey) {
      toast.error('Выберите reward asset.')
      return
    }

    setMutatingDropId('new')
    try {
      const updated = await createAdminLootboxDrop(token, detail.definition.id, {
        reward_asset_key: newDrop.rewardAssetKey,
        ...buildDropPatch(newDrop, newDrop.rewardAssetKey),
      })
      setDetail(updated)
      setDropDrafts(Object.fromEntries(updated.drops.map((item) => [item.id, draftFromDrop(item)])))
      setNewDrop(emptyNewDrop)
      toast.success('Содержимое добавлено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось добавить содержимое.'))
    } finally {
      setMutatingDropId(null)
    }
  }

  const deleteDrop = async (dropId: string) => {
    if (!detail || mutatingDropId) return
    setMutatingDropId(dropId)
    try {
      await deleteAdminLootboxDrop(token, detail.definition.id, dropId)
      const updated = await getAdminLootbox(token, detail.definition.id)
      setDetail(updated)
      setDropDrafts(Object.fromEntries(updated.drops.map((item) => [item.id, draftFromDrop(item)])))
      toast.success('Содержимое удалено.')
    } catch (cause) {
      toast.error(toDisplayError(cause, 'Не удалось удалить содержимое.'))
    } finally {
      setMutatingDropId(null)
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <AdminLink className="btn btn-sm" href={`${paths.admin}?tab=lootboxes`}>
          ← К лутбоксам
        </AdminLink>
      </section>

      {!detail && !loadError ? <LoadingState title="Загружаем лутбокс" /> : null}
      {loadError ? <ErrorState title="Лутбокс недоступен" message={loadError} primaryActionLabel="Повторить" onPrimaryAction={() => void load()} /> : null}
      {detail ? (
        <>
          <section className="card admin-card">
            <div className="admin-section-head admin-detail-hero">
              <div>
                <h2 className="card-title">{detail.definition.assetDisplayName}</h2>
                <p className="card-text">
                  <code>{detail.definition.assetKey}</code> · ID: <code>{detail.definition.id}</code>
                </p>
              </div>
              <span className={`ui-badge ${detail.definition.isActive ? 'ui-badge-success' : 'ui-badge-warning'}`}>
                {detail.definition.isActive ? 'active' : 'inactive'}
              </span>
            </div>
            <dl className="admin-kv admin-kv--compact">
              <div><dt>Asset definition</dt><dd>{detail.definition.assetDefinitionId}</dd></div>
              <div><dt>Видимость лутбокса</dt><dd>{detail.definition.isPublic ? 'public' : 'hidden'}</dd></div>
              <div><dt>Drops</dt><dd>{detail.drops.length}</dd></div>
            </dl>

            <div className="admin-asset-form-grid">
              <label className="admin-shop-field">
                <span>Название</span>
                <input
                  className="ui-input"
                  value={displayName}
                  onChange={(event) => setDisplayName(event.target.value)}
                />
              </label>
              <label className="admin-shop-field">
                <span>Описание</span>
                <input
                  className="ui-input"
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                />
              </label>
            </div>

            <div className="admin-asset-flags admin-shop-form-flags">
              <label className="admin-checkbox-row">
                <input type="checkbox" checked={isPublic} onChange={(event) => setIsPublic(event.target.checked)} />
                Лутбокс публичный
              </label>
              <label className="admin-checkbox-row">
                <input type="checkbox" checked={isActive} onChange={(event) => setIsActive(event.target.checked)} />
                Definition активен
              </label>
            </div>
            <label className="admin-shop-field admin-shop-field--full">
              <span>metadata (JSON)</span>
              <textarea className="ui-input admin-shop-textarea" rows={5} value={metadataText} onChange={(event) => setMetadataText(event.target.value)} spellCheck={false} />
            </label>
            <div className="admin-shop-actions">
              <button type="button" className="btn primary" disabled={isSavingDefinition} onClick={() => void saveDefinition()}>
                {isSavingDefinition ? 'Сохраняем...' : 'Сохранить лутбокс'}
              </button>
            </div>
          </section>

          <section className="card admin-card admin-lootbox-drop-list">
            <h2 className="card-title">Содержимое лутбокса</h2>
            <div className="admin-lootbox-table" role="table" aria-label="Содержимое лутбокса">
              <div className="admin-lootbox-table-head" role="row">
                <span>Добавить</span>
                <span>Выбор ассета</span>
                <span>Количество / срок</span>
                <span>Компенсация</span>
                <span>Вес</span>
                <span>Название</span>
                <span>Действие</span>
              </div>

              {detail.drops.map((drop, index) => {
                const asset = assetsByKey.get(drop.rewardAssetKey) ?? null
                const draft = dropDrafts[drop.id]
                return (
                  <article key={drop.id} className="admin-lootbox-cell" role="row">
                    <div className="admin-lootbox-cell-add">
                      <span className="admin-lootbox-row-index">{index + 1}</span>
                    </div>
                    <div className="admin-lootbox-cell-asset">
                      <AdminAssetSelect token={token} assets={assets} value={drop.rewardAssetKey} disabled onChange={() => undefined} />
                      <small className="admin-lootbox-field-caption">
                        reward_asset_key · {drop.isActive ? 'active' : 'inactive'}
                      </small>
                    </div>
                    {asset && draft ? (
                      <>
                        <DynamicDropValueField asset={asset} draft={draft} onChange={(patch) => updateDropDraft(drop.id, patch)} />
                        <DuplicateCompensationField
                          value={draft.duplicateCompensationAmount}
                          onChange={(duplicateCompensationAmount) => updateDropDraft(drop.id, { duplicateCompensationAmount })}
                        />
                        <label className="admin-lootbox-field">
                          <input
                            className="ui-input"
                            type="number"
                            min={1}
                            value={draft.weight}
                            onChange={(event) => updateDropDraft(drop.id, { weight: event.target.value })}
                          />
                          <small>weight</small>
                        </label>
                        <label className="admin-lootbox-field">
                          <input
                            className="ui-input"
                            value={draft.title}
                            onChange={(event) => updateDropDraft(drop.id, { title: event.target.value })}
                          />
                          <small>title_i18n ru-RU</small>
                        </label>
                        <div className="admin-lootbox-cell-actions">
                          <button type="button" className="btn btn-sm" disabled={mutatingDropId === drop.id} onClick={() => void saveDrop(drop)}>
                            {mutatingDropId === drop.id ? 'Сохраняем...' : 'Сохранить'}
                          </button>
                          <button type="button" className="btn btn-sm danger" disabled={mutatingDropId === drop.id} onClick={() => void deleteDrop(drop.id)}>
                            Удалить
                          </button>
                        </div>
                      </>
                    ) : (
                      <>
                        <span className="admin-inline-muted admin-lootbox-empty-column">Ассет не найден</span>
                        <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                        <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                        <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                        <div className="admin-lootbox-cell-actions">
                          <button type="button" className="btn btn-sm danger" disabled={mutatingDropId === drop.id} onClick={() => void deleteDrop(drop.id)}>
                            Удалить
                          </button>
                        </div>
                      </>
                    )}
                  </article>
                )
              })}

              <article className="admin-lootbox-cell admin-lootbox-cell--empty" role="row">
                <div className="admin-lootbox-cell-add">
                  <span className="admin-lootbox-plus" aria-hidden>+</span>
                </div>
                <div className="admin-lootbox-cell-asset">
                  <AdminAssetSelect
                    token={token}
                    assets={assets}
                    value={newDrop.rewardAssetKey}
                    onChange={(rewardAssetKey) => setNewDrop((prev) => ({ ...prev, rewardAssetKey }))}
                  />
                  <small className="admin-lootbox-field-caption">reward_asset_key</small>
                </div>
                {assetsByKey.get(newDrop.rewardAssetKey) ? (
                  <>
                    <DynamicDropValueField
                      asset={assetsByKey.get(newDrop.rewardAssetKey)!}
                      draft={newDrop}
                      onChange={(patch) => setNewDrop((prev) => ({ ...prev, ...patch }))}
                    />
                    <DuplicateCompensationField
                      value={newDrop.duplicateCompensationAmount}
                      onChange={(duplicateCompensationAmount) => setNewDrop((prev) => ({ ...prev, duplicateCompensationAmount }))}
                    />
                    <label className="admin-lootbox-field">
                      <input
                        className="ui-input"
                        type="number"
                        min={1}
                        value={newDrop.weight}
                        onChange={(event) => setNewDrop((prev) => ({ ...prev, weight: event.target.value }))}
                      />
                      <small>weight</small>
                    </label>
                    <label className="admin-lootbox-field">
                      <input
                        className="ui-input"
                        value={newDrop.title}
                        onChange={(event) => setNewDrop((prev) => ({ ...prev, title: event.target.value }))}
                      />
                      <small>title_i18n ru-RU</small>
                    </label>
                    <div className="admin-lootbox-cell-actions">
                      <button type="button" className="btn primary btn-sm" disabled={mutatingDropId === 'new'} onClick={() => void addDrop()}>
                        {mutatingDropId === 'new' ? 'Добавляем...' : 'Добавить'}
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                    <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                    <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                    <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                    <span className="admin-inline-muted admin-lootbox-empty-column">—</span>
                  </>
                )}
              </article>
            </div>
          </section>
        </>
      ) : null}
    </div>
  )
}
function DynamicDropValueField({
  asset,
  draft,
  onChange,
}: {
  asset: AssetResponse
  draft: DropDraft
  onChange: (patch: Partial<DropDraft>) => void
}) {
  if (asset.ownershipModel === 'stackable') {
    return (
      <label className="admin-lootbox-field">
        <input
          className="ui-input"
          type="number"
          min={1}
          value={draft.amount}
          onChange={(event) => onChange({ amount: event.target.value })}
        />
        <small>amount</small>
      </label>
    )
  }

  if (asset.ownershipModel === 'expirable') {
    return (
      <label className="admin-lootbox-field">
        <input
          className="ui-input"
          type="number"
          min={1}
          value={draft.durationSeconds}
          onChange={(event) => onChange({ durationSeconds: event.target.value })}
        />
        <small>durationSeconds</small>
      </label>
    )
  }

  return (
    <div className="admin-lootbox-field admin-lootbox-field--none">
      <span className="admin-inline-muted">Не требуется</span>
      <small>entitlement</small>
    </div>
  )
}

function DuplicateCompensationField({
  value,
  onChange,
}: {
  value: string
  onChange: (value: string) => void
}) {
  return (
    <label className="admin-lootbox-field">
      <input
        className="ui-input"
        type="number"
        min={0}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      <small>duplicate compensation</small>
    </label>
  )
}
