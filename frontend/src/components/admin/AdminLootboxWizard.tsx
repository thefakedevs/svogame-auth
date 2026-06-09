import { useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react'
import toast from 'react-hot-toast'
import { toDisplayError } from '../../api/http'
import { createAdminLootbox, createAdminLootboxDrop } from '../../api/lootboxes'
import {
  createAdminAsset,
  listAllAdminAssets,
  uploadAdminAssetImage,
  type AssetResponse,
} from '../../api/inventory'
import AdminAssetSelect from './AdminAssetSelect'

type DropDraft = {
  assetKey: string
  amount: string
  duplicateCompensationAmount: string
  durationSeconds: string
  weight: string
  title: string
}

const emptyDrop: DropDraft = {
  assetKey: '',
  amount: '1',
  duplicateCompensationAmount: '',
  durationSeconds: '',
  weight: '1',
  title: '',
}

function normalizeAssetKey(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9_-]/g, '')
}

function metadataToText(value: string) {
  const trimmed = value.trim()
  return trimmed ? JSON.parse(trimmed) as unknown : {}
}

function assetSupportsDrop(asset: AssetResponse) {
  return asset.isActive
    && (asset.ownershipModel === 'stackable' || asset.ownershipModel === 'expirable' || asset.ownershipModel === 'entitlement')
}

function ensureTrailingEmptyDrop(rows: DropDraft[]) {
  const nonEmpty = rows.filter((row) => row.assetKey)
  return [...nonEmpty, { ...emptyDrop }]
}

export default function AdminLootboxWizard({
  token,
  onCreated,
}: {
  token: string
  onCreated: (lootboxId: string) => void
}) {
  const fileInputRef = useRef<HTMLInputElement | null>(null)
  const [assets, setAssets] = useState<AssetResponse[]>([])
  const [assetsError, setAssetsError] = useState('')
  const [assetKey, setAssetKey] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [description, setDescription] = useState('')
  const [metadataText, setMetadataText] = useState('{}')
  const [isAssetPublic, setIsAssetPublic] = useState(true)
  const [isLootboxActive, setIsLootboxActive] = useState(true)
  const [imageFile, setImageFile] = useState<File | null>(null)
  const [imagePreviewUrl, setImagePreviewUrl] = useState<string | null>(null)
  const [drops, setDrops] = useState<DropDraft[]>([{ ...emptyDrop }])
  const [isSubmitting, setIsSubmitting] = useState(false)

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      setAssetsError('')
      try {
        const loaded = await listAllAdminAssets(token, 100)
        if (!cancelled) setAssets(loaded.filter(assetSupportsDrop))
      } catch (cause) {
        if (!cancelled) setAssetsError(toDisplayError(cause, 'Не удалось загрузить ассеты для дропа.'))
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [token])

  useEffect(() => {
    return () => {
      if (imagePreviewUrl) URL.revokeObjectURL(imagePreviewUrl)
    }
  }, [imagePreviewUrl])

  const assetsByKey = useMemo(() => new Map(assets.map((asset) => [asset.key, asset])), [assets])
  const selectedDrops = drops.filter((drop) => drop.assetKey)

  const onPickFile = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null
    event.target.value = ''
    setImageFile(null)
    setImagePreviewUrl((prev) => {
      if (prev) URL.revokeObjectURL(prev)
      return null
    })
    if (!file) return
    if (!file.type.startsWith('image/')) {
      toast.error('Нужен файл изображения.')
      return
    }
    if (file.size > 2 * 1024 * 1024) {
      toast.error('Изображение не больше 2 МБ.')
      return
    }
    setImageFile(file)
    setImagePreviewUrl(URL.createObjectURL(file))
  }

  const updateDrop = (index: number, patch: Partial<DropDraft>) => {
    setDrops((prev) => ensureTrailingEmptyDrop(prev.map((row, rowIndex) => (rowIndex === index ? { ...row, ...patch } : row))))
  }

  const removeDrop = (index: number) => {
    setDrops((prev) => ensureTrailingEmptyDrop(prev.filter((_, rowIndex) => rowIndex !== index)))
  }

  const submit = async () => {
    const key = assetKey.trim()
    const name = displayName.trim()
    if (!key || key !== normalizeAssetKey(key)) {
      toast.error('Key лутбокса: только строчные латинские буквы, цифры, _ и -.')
      return
    }
    if (!name) {
      toast.error('Укажите название лутбокса.')
      return
    }
    if (!selectedDrops.length) {
      toast.error('Добавьте хотя бы один ассет в содержимое.')
      return
    }

    let metadata: unknown
    try {
      metadata = metadataToText(metadataText)
    } catch {
      toast.error('Metadata должен быть валидным JSON.')
      return
    }

    let normalizedDrops: Array<{
      drop: DropDraft
      asset: AssetResponse
      amount: number | null
      durationSeconds: number | null
      duplicateCompensationAmount: number | null
      weight: number
      sortOrder: number
    }>
    try {
      normalizedDrops = selectedDrops.map((drop, index) => {
        const asset = assetsByKey.get(drop.assetKey)
        if (!asset) throw new Error(`Ассет ${drop.assetKey} не найден.`)
        const weight = Number.parseInt(drop.weight, 10)
        if (!Number.isFinite(weight) || weight <= 0) throw new Error(`Вес в ячейке ${index + 1} должен быть больше 0.`)
        const duplicateCompensationAmount = drop.duplicateCompensationAmount.trim()
          ? Number.parseInt(drop.duplicateCompensationAmount, 10)
          : null
        if (duplicateCompensationAmount !== null && (!Number.isFinite(duplicateCompensationAmount) || duplicateCompensationAmount < 0)) {
          throw new Error(`Компенсация дубля в ячейке ${index + 1} должна быть числом не меньше 0 или пустой.`)
        }

        if (asset.ownershipModel === 'stackable') {
          const amount = Number.parseInt(drop.amount, 10)
          if (!Number.isFinite(amount) || amount <= 0) throw new Error(`Количество в ячейке ${index + 1} должно быть больше 0.`)
          return { drop, asset, amount, durationSeconds: null, duplicateCompensationAmount, weight, sortOrder: index }
        }

        if (asset.ownershipModel === 'entitlement') {
          return { drop, asset, amount: null, durationSeconds: null, duplicateCompensationAmount, weight, sortOrder: index }
        }

        const durationSeconds = Number.parseInt(drop.durationSeconds, 10)
        if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) {
          throw new Error(`durationSeconds в ячейке ${index + 1} должен быть больше 0.`)
        }
        return { drop, asset, amount: null, durationSeconds, duplicateCompensationAmount, weight, sortOrder: index }
      })
    } catch (cause) {
      toast.error(cause instanceof Error ? cause.message : 'Проверьте содержимое лутбокса.')
      return
    }

    setIsSubmitting(true)
    let createdAssetId: string | null = null
    try {
      const createdAsset = await createAdminAsset(token, {
        key,
        display_name: name,
        description: description.trim() || null,
        asset_kind: 'lootbox',
        ownership_model: 'stackable',
        is_currency: false,
        is_user_purchasable: true,
        is_public: isAssetPublic,
        metadata,
      })
      createdAssetId = createdAsset.id
      if (imageFile) await uploadAdminAssetImage(token, createdAsset.id, imageFile)

      const lootbox = await createAdminLootbox(token, {
        asset_key: key,
        is_active: isLootboxActive,
        metadata,
      })

      let latest = lootbox
      for (const item of normalizedDrops) {
        latest = await createAdminLootboxDrop(token, latest.definition.id, {
          reward_asset_key: item.asset.key,
          amount: item.amount,
          duplicate_compensation_amount: item.duplicateCompensationAmount,
          duration_seconds: item.durationSeconds,
          weight: item.weight,
          title_i18n: item.drop.title.trim() ? { 'ru-RU': item.drop.title.trim() } : {},
          is_active: true,
          sort_order: item.sortOrder,
        })
      }

      onCreated(latest.definition.id)
    } catch (cause) {
      const hint = createdAssetId ? ` Ассет лутбокса уже создан: ${createdAssetId}.` : ''
      toast.error(toDisplayError(cause, 'Не удалось создать лутбокс.') + hint)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <section className="card admin-card admin-lootbox-wizard" aria-label="Полный цикл создания лутбокса">
      <h2 className="card-title">Новый лутбокс одной кнопкой</h2>
      <p className="admin-inline-muted">
        Создаёт ассет <code>lootbox</code> со stackable-владением, объявляет его лутбоксом и добавляет выбранные ассеты как drops.
        В содержимое можно добавлять активные stackable, expirable и entitlement ассеты, включая валюту.
      </p>

      {assetsError ? <p className="error-message">{assetsError}</p> : null}

      <div className="admin-asset-form-grid">
        <input
          className="ui-input"
          value={assetKey}
          onChange={(event) => setAssetKey(normalizeAssetKey(event.target.value))}
          placeholder="key лутбокса"
          autoComplete="off"
        />
        <input
          className="ui-input"
          value={displayName}
          onChange={(event) => setDisplayName(event.target.value)}
          placeholder="Название"
        />
      </div>
      <textarea
        className="ui-input admin-asset-metadata"
        value={description}
        onChange={(event) => setDescription(event.target.value)}
        placeholder="Описание"
        rows={2}
      />

      <div className="admin-skin-wizard__image-row">
        <input
          ref={fileInputRef}
          className="admin-hidden-file-input"
          type="file"
          accept="image/png,image/jpeg,image/webp,image/*"
          onChange={onPickFile}
        />
        <button type="button" className="btn btn-sm" onClick={() => fileInputRef.current?.click()}>
          Превью лутбокса
        </button>
        {imagePreviewUrl ? (
          <span className="admin-skin-wizard__thumb-wrap">
            <img src={imagePreviewUrl} alt="" className="admin-skin-wizard__thumb" />
          </span>
        ) : (
          <span className="admin-inline-muted">Опционально: PNG, JPG или WebP до 2 МБ.</span>
        )}
      </div>

      <div className="admin-asset-flags">
        <label className="admin-checkbox-row">
          <input type="checkbox" checked={isAssetPublic} onChange={(event) => setIsAssetPublic(event.target.checked)} />
          Ассет публичный
        </label>
        <label className="admin-checkbox-row">
          <input type="checkbox" checked={isLootboxActive} onChange={(event) => setIsLootboxActive(event.target.checked)} />
          Лутбокс активен
        </label>
      </div>

      <label className="admin-shop-field admin-shop-field--full">
        <span>metadata (JSON)</span>
        <textarea
          className="ui-input admin-shop-textarea"
          rows={4}
          value={metadataText}
          onChange={(event) => setMetadataText(event.target.value)}
          spellCheck={false}
        />
      </label>

      <div className="admin-lootbox-builder">
        <div className="admin-shop-locales-head">
          <strong>Содержимое</strong>
          <span className="admin-inline-muted">Заполненная ячейка автоматически добавляет следующую с плюсом.</span>
        </div>
        <div className="admin-lootbox-table" role="table" aria-label="Содержимое нового лутбокса">
          <div className="admin-lootbox-table-head" role="row">
            <span>Номер</span>
            <span>Выбор ассета</span>
            <span>Количество / срок</span>
            <span>Компенсация</span>
            <span>Вес</span>
            <span>Название</span>
            <span>Действие</span>
          </div>
          {drops.map((drop, index) => {
            const selectedAsset = assetsByKey.get(drop.assetKey) ?? null
            const isEmptyCell = !drop.assetKey
            return (
              <article key={index} className={`admin-lootbox-cell ${isEmptyCell ? 'admin-lootbox-cell--empty' : ''}`} role="row">
                <div className="admin-lootbox-cell-add">
                  {isEmptyCell ? <span className="admin-lootbox-plus" aria-hidden>+</span> : <span className="admin-lootbox-row-index">{index + 1}</span>}
                </div>
                <div className="admin-lootbox-cell-asset">
                  <AdminAssetSelect token={token} assets={assets} value={drop.assetKey} onChange={(assetKey) => updateDrop(index, { assetKey })} />
                  <small className="admin-lootbox-field-caption">reward_asset_key</small>
                </div>
                {selectedAsset ? (
                  <>
                    {selectedAsset.ownershipModel === 'stackable' ? (
                      <label className="admin-lootbox-field">
                        <input
                          className="ui-input"
                          type="number"
                          min={1}
                          value={drop.amount}
                          onChange={(event) => updateDrop(index, { amount: event.target.value })}
                        />
                        <small>amount</small>
                      </label>
                    ) : selectedAsset.ownershipModel === 'expirable' ? (
                      <label className="admin-lootbox-field">
                        <input
                          className="ui-input"
                          type="number"
                          min={1}
                          value={drop.durationSeconds}
                          onChange={(event) => updateDrop(index, { durationSeconds: event.target.value })}
                        />
                        <small>durationSeconds</small>
                      </label>
                    ) : (
                      <div className="admin-lootbox-field admin-lootbox-field--none">
                        <span className="admin-inline-muted">Не требуется</span>
                        <small>entitlement</small>
                      </div>
                    )}
                    <label className="admin-lootbox-field">
                      <input
                        className="ui-input"
                        type="number"
                        min={0}
                        value={drop.duplicateCompensationAmount}
                        onChange={(event) => updateDrop(index, { duplicateCompensationAmount: event.target.value })}
                      />
                      <small>duplicate compensation</small>
                    </label>
                    <label className="admin-lootbox-field">
                      <input
                        className="ui-input"
                        type="number"
                        min={1}
                        value={drop.weight}
                        onChange={(event) => updateDrop(index, { weight: event.target.value })}
                      />
                      <small>weight</small>
                    </label>
                    <label className="admin-lootbox-field">
                      <input
                        className="ui-input"
                        value={drop.title}
                        onChange={(event) => updateDrop(index, { title: event.target.value })}
                      />
                      <small>title_i18n ru-RU</small>
                    </label>
                    <div className="admin-lootbox-cell-actions">
                      <button type="button" className="btn btn-sm danger" onClick={() => removeDrop(index)}>
                        Удалить
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
            )
          })}
        </div>
      </div>

      <div className="admin-skin-wizard__actions">
        <button type="button" className="btn primary" disabled={isSubmitting || Boolean(assetsError)} onClick={() => void submit()}>
          {isSubmitting ? 'Создаём...' : 'Создать ассет, лутбокс и содержимое'}
        </button>
      </div>
    </section>
  )
}
