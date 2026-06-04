import { ApiError, authHeaders, request } from "./http";

export interface UserResponse {
  id: string;
  discordId: string;
  username: string;
  avatarUrl: string | null;
  email: string | null;
  isActive: boolean;
  isSuperuser: boolean;
  lastLoginAt: string;
  createdAt: string;
}

export function getCurrentUser(token: string): Promise<UserResponse> {
  return request<UserResponse>("/api/user/me", {
    method: "GET",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
  });
}

export function updateNickname(token: string, nickname: string): Promise<UserResponse> {
  return request<UserResponse>("/api/user/me/nickname", {
    method: "POST",
    headers: authHeaders(token, {
      "Content-Type": "application/json",
    }),
    body: JSON.stringify({ nickname }),
  });
}

export { ApiError };
