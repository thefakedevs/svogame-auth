import { ApiError } from './userApi'

export type SkinModel = 'default' | 'slim'

export interface UploadSkinResponse {
  status: string
  uuid: string
  model: SkinModel
  message: string
}

export function buildSkinUrl(userId: string, version?: number): string {
  const url = `/api/skins/${userId}`
  return version ? `${url}?v=${version}` : url
}

export async function uploadMySkin(
  token: string,
  file: File,
  model: SkinModel,
): Promise<UploadSkinResponse> {
  const formData = new FormData()
  formData.append('file', file)

  const response = await fetch(`/api/skins/me?model=${model}`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
    },
    body: formData,
  })

  if (!response.ok) {
    let errorMessage = response.statusText
    try {
      const errorBody = await response.json()
      if (errorBody?.message) {
        errorMessage = errorBody.message
      }
    } catch {
      // ignore parse errors
    }

    throw new ApiError(response.status, response.statusText, errorMessage)
  }

  return response.json()
}
