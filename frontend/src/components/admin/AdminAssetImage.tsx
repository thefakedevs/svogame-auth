import { useEffect, useState } from 'react'
import {
  fetchAdminAssetImageObjectUrl,
  type AssetResponse,
} from '../../api/inventory'

type AdminAssetImageProps = {
  token: string
  asset: AssetResponse | null
  reloadKey?: number
  className?: string
  placeholder?: string
  alt?: string
}

export default function AdminAssetImage({
  token,
  asset,
  reloadKey = 0,
  className = '',
  placeholder = 'Нет изображения',
  alt = '',
}: AdminAssetImageProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(asset?.imageUrl ?? null)
  const [isLoading, setIsLoading] = useState(false)
  const [isImageFailed, setIsImageFailed] = useState(false)

  useEffect(() => {
    if (!asset) {
      setImageUrl(null)
      setIsLoading(false)
      setIsImageFailed(false)
      return undefined
    }

    let isActive = true
    let objectUrl: string | null = null

    setIsLoading(true)
    setIsImageFailed(false)

    fetchAdminAssetImageObjectUrl(token, asset.id, `${asset.updatedAt}-${reloadKey}`)
      .then((url) => {
        if (!isActive) {
          if (url) URL.revokeObjectURL(url)
          return
        }

        objectUrl = url
        setImageUrl(url ?? asset.imageUrl ?? null)
        setIsImageFailed(false)
      })
      .catch(() => {
        if (!isActive) return
        const fallbackUrl = asset.imageUrl ?? null
        setImageUrl(fallbackUrl)
        setIsImageFailed(!fallbackUrl)
      })
      .finally(() => {
        if (isActive) setIsLoading(false)
      })

    return () => {
      isActive = false
      if (objectUrl) URL.revokeObjectURL(objectUrl)
    }
  }, [asset, reloadKey, token])

  const showImage = Boolean(imageUrl) && !isImageFailed
  const rootClassName = ['admin-asset-image-preview', className].filter(Boolean).join(' ')

  return (
    <div className={rootClassName}>
      {showImage ? (
        <img src={imageUrl ?? ''} alt={alt} loading="lazy" onError={() => setIsImageFailed(true)} />
      ) : (
        <span>{isLoading ? 'Загрузка...' : placeholder}</span>
      )}
    </div>
  )
}
