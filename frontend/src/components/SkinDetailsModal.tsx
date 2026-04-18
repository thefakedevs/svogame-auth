import { useState, type CSSProperties, type ReactNode } from 'react'
import type { SkinRarity } from '../api/inventory'
import './SkinDetailsModal.css'

export type SkinDetailsModalItem = {
  title: string
  description: string
  imageUrl: string | null
  accent: string
  metaItems?: Array<{ label: string; value: string }>
  rarity?: SkinRarity | null
  weaponKey?: string | null
  priceText?: string | null
  statusText?: string | null
}

const rarityLabels: Record<SkinRarity, string> = {
  common: 'Обычный',
  rare: 'Редкий',
  legendary: 'Легендарный',
}

function cardStyle(accent: string): CSSProperties {
  return { '--inventory-accent': accent } as CSSProperties
}

function rarityLabel(rarity: SkinRarity | null | undefined) {
  return rarity ? rarityLabels[rarity] : 'Скин'
}

function defaultMetaItems(item: SkinDetailsModalItem) {
  const rows: Array<{ label: string; value: string }> = []

  if (item.priceText) {
    rows.push({ label: 'Цена', value: item.priceText })
  }

  rows.push({ label: 'Редкость', value: rarityLabel(item.rarity) })

  if (item.weaponKey) {
    rows.push({ label: 'Оружие', value: item.weaponKey })
  }

  return rows
}

function SkinDetailsVisual({ item }: { item: SkinDetailsModalItem }) {
  const [failedImageUrl, setFailedImageUrl] = useState<string | null>(null)
  const fallback = item.title.trim().slice(0, 1).toUpperCase() || 'S'
  const showImage = Boolean(item.imageUrl) && failedImageUrl !== item.imageUrl

  return (
    <div className="skin-details-modal__visual" style={cardStyle(item.accent)}>
      {showImage ? <img src={item.imageUrl ?? ''} alt="" loading="lazy" onError={() => setFailedImageUrl(item.imageUrl)} /> : <span>{fallback}</span>}
      <div className="inventory-visual-splash" />
    </div>
  )
}

export default function SkinDetailsModal({
  item,
  titleId,
  isBusy,
  footer,
  onClose,
}: {
  item: SkinDetailsModalItem
  titleId: string
  isBusy?: boolean
  footer: ReactNode
  onClose: () => void
}) {
  return (
    <div className="ui-modal-backdrop" role="presentation" onClick={() => !isBusy && onClose()}>
      <div className="ui-modal skin-details-modal" role="dialog" aria-modal="true" aria-labelledby={titleId} onClick={(event) => event.stopPropagation()}>
        <div className="ui-modal-header">
          <h2 id={titleId} className="ui-modal-title">{item.title}</h2>
          <button className="ui-modal-close" type="button" aria-label="Закрыть" onClick={onClose} disabled={isBusy}>
            ×
          </button>
        </div>
        <div className="ui-modal-body">
          <div className="skin-details-modal__body">
            <SkinDetailsVisual item={item} />
            <div className="skin-details-modal__content">
              <p>{item.description}</p>
              <dl className="skin-details-modal__meta">
                {(item.metaItems ?? defaultMetaItems(item)).map((metaItem) => (
                  <div key={`${metaItem.label}:${metaItem.value}`}>
                    <dt>{metaItem.label}</dt>
                    <dd>{metaItem.value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          </div>
        </div>
        <div className="ui-modal-footer">{footer}</div>
      </div>
    </div>
  )
}
