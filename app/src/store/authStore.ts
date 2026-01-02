import {create} from 'zustand'
import type {UserProfile} from '../services/authApi'
import { persist } from 'zustand/middleware';

interface AuthState {
    user: UserProfile | null
    powData: { solution: string, prefix: string } | null
    isAuthenticated: boolean
    setUser: (user: UserProfile | null) => void
    setPoWData: (data: { solution: string, prefix: string } | null) => void
    logout: () => void
}

export const useAuthStore = create<AuthState>()(
    persist(
        (set) => ({
            user: null,
            powData: null,
            isAuthenticated: false,
            setUser: (user) => set({user, isAuthenticated: !!user}),
            setPoWData: (data: { solution: string, prefix: string } | null) => set({powData: data}),
            logout: () => set({user: null, powData: null, isAuthenticated: false}),
        }), {name: "auth-storage"}
    )
)

