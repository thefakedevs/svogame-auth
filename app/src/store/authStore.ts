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

// Main store for user/token - persisted to localStorage
export const useAuthStore = create<AuthState>()(
    persist(
        (set) => ({
            user: null,
            powData: null,
            token: null,
            setUser: (user) => set({user}),
            setPoWData: (data: { solution: string, prefix: string } | null) => {
                // Save powData to sessionStorage (survives redirects, cleared on tab close)
                if (data) {
                    sessionStorage.setItem('pow-data', JSON.stringify(data))
                } else {
                    sessionStorage.removeItem('pow-data')
                }
                set({powData: data})
            },
            setToken: (token) => set({token}),
        }), {
            name: "auth-storage",
            // Exclude powData from localStorage persistence
            partialize: (state) => ({ user: state.user, token: state.token }),
            onRehydrateStorage: () => (state?: AuthState) => {
                // Restore powData from sessionStorage on page load
                if (state) {
                    try {
                        const stored = sessionStorage.getItem('pow-data')
                        if (stored) {
                            state.powData = JSON.parse(stored)
                        }
                    } catch (e) {
                        // ignore parse errors
                    }
                }
            }
        }
    )
)

