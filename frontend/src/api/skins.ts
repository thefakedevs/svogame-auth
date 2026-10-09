import { ApiError, authHeaders, request } from './http'
import { STATIC_PREVIEW } from './staticPreview'

export type SkinModel = 'default' | 'slim'

export interface UploadSkinResponse {
  status: string
  uuid: string
  model: SkinModel
  message: string
}

export function buildSkinUrl(userId: string, version?: number): string {
  if (STATIC_PREVIEW) return '/static-preview/default-skin.png'
  const url = `/api/skins/${userId}`
  return version ? `${url}?v=${version}` : url
}

export function uploadMySkin(
  token: string,
  file: File,
  model: SkinModel,
): Promise<UploadSkinResponse> {
  const formData = new FormData()
  formData.append('file', file)

  return request<UploadSkinResponse>(`/api/skins/me?model=${model}`, {
    method: 'POST',
    headers: authHeaders(token),
    body: formData,
  })
}

export { ApiError }
