import { useRef, useState } from 'react'
import toast from 'react-hot-toast'

import SkinViewer3D from './SkinViewer3D'
import { uploadMySkin, type SkinModel } from '../services/skinApi'
import { tokenManager } from '../services/tokenManager'
import './SkinUploadInline.css'

export default function SkinUploadInline({ onUploaded }: { onUploaded?: () => void }) {
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [model, setModel] = useState<SkinModel>('default')
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFileChange = (file: File) => {
    if (file.type.startsWith('image/')) {
      const reader = new FileReader()
      reader.onload = (e) => {
        setPreviewUrl(e.target?.result as string)
      }
      reader.readAsDataURL(file)
    }
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) handleFileChange(file)
  }

  const handleUpload = async () => {
    const file = fileInputRef.current?.files?.[0]
    if (!file) return

    const token = await tokenManager.getToken()
    if (!token) {
      toast.error('Токен авторизации не найден')
      return
    }

    setIsLoading(true)

    const uploadPromise = uploadMySkin(token, file, model)

    toast.promise(uploadPromise, {
      loading: 'Загрузка скина...',
      success: 'Скин успешно загружен!',
      error: (err) => err.message || 'Ошибка при загрузке',
    })

    try {
      await uploadPromise
      setPreviewUrl(null)
      if (fileInputRef.current) fileInputRef.current.value = ''
      onUploaded?.()
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="skin-upload-inline">
      <div className="upload-preview-3d">
        {previewUrl ? (
          <>
            <SkinViewer3D
              skinUrl={previewUrl}
              model={model}
              width={300}
              height={400}
            />
            <div className="model-toggle">
              <button
                type="button"
                className={`btn btn-sm model-btn ${model === 'default' ? 'active' : ''}`}
                onClick={() => setModel('default')}
              >
                Обычные
              </button>
              <button
                type="button"
                className={`btn btn-sm model-btn ${model === 'slim' ? 'active' : ''}`}
                onClick={() => setModel('slim')}
              >
                Тонкие
              </button>
            </div>
          </>
        ) : (
          <div className="upload-placeholder" onClick={() => fileInputRef.current?.click()}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
            <span>Выберите скин</span>
          </div>
        )}
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        onChange={handleInputChange}
        style={{ display: 'none' }}
      />

      {previewUrl && (
        <div className="upload-buttons">
          <button
            type="button"
            className="btn btn-success"
            onClick={handleUpload}
            disabled={isLoading}
          >
            {isLoading ? 'Загрузка...' : 'Загрузить'}
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => {
              setPreviewUrl(null)
              if (fileInputRef.current) fileInputRef.current.value = ''
            }}
            disabled={isLoading}
          >
            Очистить
          </button>
        </div>
      )}
    </div>
  )
}
