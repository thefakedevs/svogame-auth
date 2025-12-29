import {create} from 'zustand'
import type {UserProfile} from '../services/authApi'
import { persist } from 'zustand/middleware';

interface AuthState {
    user: UserProfile | null
    powData: { solution: string, prefix: string } | null
    setUser: (user: UserProfile | null) => void
    setPoWData: (data: { solution: string, prefix: string } | null) => void
}

export const useAuthStore = create<AuthState>()(
    persist(
        (set) => ({
            user: null,
            powData: null,
            setUser: (user) => set({user}),
            setPoWData: (data: { solution: string, prefix: string } | null) => set({powData: data}),
        }), {name: "auth-storage"}
    )
)

