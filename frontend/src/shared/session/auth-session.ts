import type { UserProfile } from "../../api/auth";
import { useAuthStore } from "../../store/authStore";
import { loadStoredPowData, saveStoredPowData } from "./pow-storage";
import type { PowData } from "./token";

export function bootstrapAuthSession() {
  const powData = loadStoredPowData();
  if (powData) {
    useAuthStore.setState({ powData });
  }
}

export function getAuthToken() {
  return useAuthStore.getState().token;
}

export function setAuthToken(token: string | null) {
  useAuthStore.getState().setToken(token);
}

export function setAuthUser(user: UserProfile | null) {
  useAuthStore.getState().setUser(user);
}

export function getPowData() {
  return useAuthStore.getState().powData;
}

export function setPowData(data: PowData | null) {
  saveStoredPowData(data);
  useAuthStore.getState().setPoWData(data);
}

export function clearAuthSession() {
  setPowData(null);
  setAuthToken(null);
  setAuthUser(null);
}

export function storeAuthorizedSession(input: { token: string; user: UserProfile }) {
  setAuthToken(input.token);
  setAuthUser(input.user);
}
