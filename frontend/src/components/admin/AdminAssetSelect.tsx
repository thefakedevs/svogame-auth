import { useMemo, useState } from 'react'
import type { AssetResponse } from '../../api/inventory'
import AdminAssetImage from './AdminAssetImage'

type AdminAssetSelectProps = {
  token: string
  assets: AssetResponse[]
  value: string
  onChange: (assetKey: string, asset: AssetResponse | null) => void
  disabled?: boolean
  placeholder?: string
  emptyText?: string
  filterAsset?: (asset: AssetResponse) => boolean
}

function assetMeta(asset: AssetResponse) {
  const flags = [
    asset.ownershipModel,
    asset.assetKind,
    asset.isCurrency ? 'currency' : null,
    asset.isActive ? 'active' : 'inactive',
    asset.isPublic ? 'public' : 'hidden',
  ].filter(Boolean)

  return flags.join(' · ')
}

function assetSearchText(asset: AssetResponse) {
  return [
    asset.key,
    asset.displayName,
    asset.description,
    asset.assetKind,
    asset.ownershipModel,
    asset.rarity,
    asset.weaponKey,
  ].filter(Boolean).join(' ').toLowerCase()
}

export default function AdminAssetSelect({
  token,
  assets,
  value,
  onChange,
  disabled = false,
  placeholder = 'Поиск ассета',
  emptyText = 'Ассеты не найдены.',
  filterAsset,
}: AdminAssetSelectProps) {
  const [query, setQuery] = useState('')
  const [isOpen, setIsOpen] = useState(false)

  const availableAssets = useMemo(
    () => filterAsset ? assets.filter(filterAsset) : assets,
    [assets, filterAsset],
  )
  const selectedAsset = useMemo(
    () => assets.find((asset) => asset.key === value) ?? null,
    [assets, value],
  )
  const visibleAssets = useMemo(() => {
    const needle = query.trim().toLowerCase()
    const filtered = needle
      ? availableAssets.filter((asset) => assetSearchText(asset).includes(needle))
      : availableAssets
    return filtered.slice(0, 30)
  }, [availableAssets, query])

  const selectAsset = (asset: AssetResponse) => {
    onChange(asset.key, asset)
    setQuery('')
    setIsOpen(false)
  }

  return (
    <div className="admin-picker admin-asset-picker">
      {selectedAsset ? (
        <div className="admin-picker-selected">
          <AdminAssetImage token={token} asset={selectedAsset} className="admin-asset-image-preview--thumb" alt={selectedAsset.displayName} />
          <span className="admin-picker-selected__text">
            <strong>{selectedAsset.displayName}</strong>
            <small>{selectedAsset.key} · {assetMeta(selectedAsset)}</small>
          </span>
          <button type="button" className="btn btn-sm" disabled={disabled} onClick={() => onChange('', null)}>
            Сбросить
          </button>
        </div>
      ) : null}
      {!selectedAsset ? (
        <input
          className="ui-input"
          value={query}
          disabled={disabled}
          onFocus={() => setIsOpen(true)}
          onChange={(event) => {
            setQuery(event.target.value)
            setIsOpen(true)
          }}
          placeholder={placeholder}
        />
      ) : null}
      {isOpen && !disabled ? (
        <div className="admin-picker-menu">
          {visibleAssets.map((asset) => (
            <button
              key={asset.id}
              type="button"
              className={`admin-picker-option ${asset.key === value ? 'is-active' : ''}`}
              onMouseDown={(event) => {
                event.preventDefault()
                event.stopPropagation()
                selectAsset(asset)
              }}
              onClick={() => selectAsset(asset)}
            >
              <AdminAssetImage token={token} asset={asset} className="admin-asset-image-preview--thumb" alt={asset.displayName} />
              <span className="admin-picker-option__text">
                <strong>{asset.displayName}</strong>
                <small>{asset.key} · {assetMeta(asset)}</small>
                {asset.description ? <small>{asset.description}</small> : null}
              </span>
            </button>
          ))}
          {visibleAssets.length === 0 ? <p className="admin-inline-muted admin-picker-empty">{emptyText}</p> : null}
        </div>
      ) : null}
    </div>
  )
}
