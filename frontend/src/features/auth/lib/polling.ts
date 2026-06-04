import type { DiscordAuthInitResponse } from "../../../api/auth";

export function parsePollingData(pollingData: string): DiscordAuthInitResponse {
  const parsed = JSON.parse(atob(pollingData)) as DiscordAuthInitResponse;

  if (!parsed.oauthUrl || !parsed.powPrefix || !Number.isFinite(parsed.powComplexity)) {
    throw new Error("Некорректные параметры авторизации.");
  }

  return parsed;
}
