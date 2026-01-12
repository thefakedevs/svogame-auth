import { useState, useRef } from 'react'
import toast from 'react-hot-toast'
import SkinViewer3D from './SkinViewer3D'
import { tokenManager } from '../services/tokenManager'
import './SkinUploadInline.css'

type SkinModel = 'classic' | 'slim'

export default function SkinUploadInline({ userUuid }: { userUuid: string }) {
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [model, setModel] = useState<SkinModel>('classic')
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

    const uploadPromise = (async () => {
      const formData = new FormData()
      formData.append('', file)

      const response = await fetch(`http://localhost:80/skin/${userUuid}?model=${model}`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`
        },
        body: formData
      })

      if (!response.ok) {
        throw new Error(`Ошибка загрузки: ${response.statusText}`)
      }

      return response
    })()

    toast.promise(
      uploadPromise,
      {
        loading: 'Загрузка скина...',
        success: 'Скин успешно загружен!',
        error: (err) => err.message || 'Ошибка при загрузке'
      }
    )

    try {
      await uploadPromise
      setPreviewUrl(null)
      if (fileInputRef.current) fileInputRef.current.value = ''
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
                className={`model-btn ${model === 'classic' ? 'active' : ''}`}
                onClick={() => setModel('classic')}
              >
                Обычные
              </button>
              <button
                type="button"
                className={`model-btn ${model === 'slim' ? 'active' : ''}`}
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
            className="btn-upload"
            onClick={handleUpload}
            disabled={isLoading}
          >
            {isLoading ? 'Загрузка...' : 'Загрузить'}
          </button>
          <button
            type="button"
            className="btn-clear"
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
