import { request } from "./http";

export interface DiscordGuildEventResponse {
  id: string;
  name: string;
  description: string | null;
  status: string;
  startsAt: string;
  endsAt: string | null;
  imageUrl: string | null;
  location: string | null;
  userCount: number | null;
}

export function listDiscordGuildEvents(): Promise<DiscordGuildEventResponse[]> {
  return request<DiscordGuildEventResponse[]>("/api/discord/events");
}
