import type { ReactNode } from "react";
import type { ProfileDashboardData } from "../../../components/profile/types";
import type { SquadViewMode } from "../types";

const dateTimeFormatter = new Intl.DateTimeFormat("ru-RU", {
  day: "2-digit",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

const dateFormatter = new Intl.DateTimeFormat("ru-RU", {
  day: "2-digit",
  month: "short",
  year: "numeric",
});

export function resolveSquadViewMode(data: ProfileDashboardData): SquadViewMode {
  if (!data.squad) return "no_squad";
  return data.squad.leaderUserId === data.user.id ? "leader" : "member";
}

export function initials(value: string) {
  return value.slice(0, 2).toUpperCase();
}

export function formatDate(value?: string | null) {
  if (!value) return "Нет данных";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : dateFormatter.format(date).replace(", ", " ");
}

export function formatDateTime(value?: string | null) {
  if (!value) return "Нет данных";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(", ", " ");
}

export function renderUserAvatar(user: { username: string; avatarUrl?: string | null }): ReactNode {
  return user.avatarUrl ? (
    <img
      className="profile-member-avatar profile-member-avatar--sm"
      src={user.avatarUrl}
      alt={user.username}
    />
  ) : (
    <span className="profile-member-avatar profile-member-avatar--sm">
      {initials(user.username)}
    </span>
  );
}

export function formatImageLimit(bytes: number) {
  const megabytes = bytes / (1024 * 1024);
  if (Number.isInteger(megabytes)) return `${megabytes} МБ`;
  return `${megabytes.toFixed(1).replace(".", ",")} МБ`;
}
