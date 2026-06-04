import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { UserProfile } from "../api/auth";
import type { PowData } from "../shared/session/token";

interface AuthState {
  user: UserProfile | null;
  powData: PowData | null;
  token: string | null;
  hydrated: boolean;
  setUser: (user: UserProfile | null) => void;
  setPoWData: (data: PowData | null) => void;
  setToken: (token: string | null) => void;
  setHydrated: (hydrated: boolean) => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      user: null,
      powData: null,
      token: null,
      hydrated: false,
      setUser: (user) => set({ user }),
      setPoWData: (powData) => set({ powData }),
      setToken: (token) => set({ token }),
      setHydrated: (hydrated) => set({ hydrated }),
    }),
    {
      name: "auth-storage",
      partialize: (state) => ({ user: state.user, token: state.token }),
      onRehydrateStorage: () => (state) => {
        state?.setHydrated(true);
      },
    },
  ),
);
