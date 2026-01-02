import { getAuthToken } from '../util/tokenStorage'

export interface User {
    id: string
    discordId: string
    username: string
    avatarUrl: string | null
    email: string | null
    isActive: boolean
    lastLoginAt: string
    createdAt: string
}

export interface UpdateNicknameRequest {
    nickname: string
}

export async function getCurrentUser(): Promise<User> {
    const token = getAuthToken()
    if (!token) {
        throw new Error('Не найден токен авторизации')
    }

    const resp = await fetch('/api/user/me', {
        method: 'GET',
        headers: {
            'Authorization': `Bearer ${token}`,
            'Content-Type': 'application/json',
        },
    })

    const data = await resp.json()

    if (!resp.ok) {
        const errorMessage = data.error || `${resp.status} ${resp.statusText}`
        throw new Error(`Ошибка при получении данных пользователя: ${errorMessage}`)
    }

    return data
}

export async function updateUserNickname(nickname: string): Promise<User> {
    const token = getAuthToken()
    if (!token) {
        throw new Error('Не найден токен авторизации')
    }

    const resp = await fetch('/api/user/me/nickname', {
        method: 'POST',
        headers: {
            'Authorization': `Bearer ${token}`,
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ nickname }),
    })

    const data = await resp.json()

    if (!resp.ok) {
        const errorMessage = data.error || `${resp.status} ${resp.statusText}`
        throw new Error(`Ошибка при обновлении никнейма: ${errorMessage}`)
    }

    return data
}