import {create} from 'zustand'
import type {UserProfile} from '../services/authApi'
import { persist } from 'zustand/middleware'

interface AuthState {
    user: UserProfile | null
    powData: { solution: string, prefix: string } | null
    token: string | null
    setUser: (user: UserProfile | null) => void
    setPoWData: (data: { solution: string, prefix: string } | null) => void
    setToken: (token: string | null) => void
}

export const useAuthStore = create<AuthState>()(
    persist(
        (set) => ({
            user: null,
            powData: null,
            token: null,
            setUser: (user) => set({user}),
            setPoWData: (data: { solution: string, prefix: string } | null) => set({powData: data}),
            setToken: (token) => set({token}),
        }), {name: "auth-storage"}
    )
)

