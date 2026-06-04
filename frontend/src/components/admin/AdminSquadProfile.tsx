import { useEffect, useRef, useState } from "react";
import toast from "react-hot-toast";
import {
  deleteAdminSquad,
  deleteAdminSquadImage,
  kickAdminSquadMember,
  patchAdminSquad,
  restrictAdminSquad,
  unrestrictAdminSquad,
  uploadAdminSquadImage,
  type AdminSquadResponse,
} from "../../api/admin";
import { toDisplayError } from "../../api/http";
import type { SquadMemberResponse } from "../../api/squads";
import { adminUserPath, paths } from "../../routes/paths";
import { pushUrl } from "../../shared/navigation/history";
import AppPortal from "../../shared/ui/portal/AppPortal";
import LoadingState from "../LoadingState";
import AdminLink from "./AdminLink";

function formatDateTime(value?: string | null) {
  if (!value) return "Нет данных";
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("ru-RU", {
        day: "2-digit",
        month: "short",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date);
}

function initials(value: string) {
  return value.slice(0, 2).toUpperCase();
}

function appendVersion(url: string, version: number) {
  return `${url}${url.includes("?") ? "&" : "?"}v=${version}`;
}

export default function AdminSquadProfile({
  token,
  squad,
  members,
  onSquadChange,
  onMembersChange,
}: {
  token: string;
  squad: AdminSquadResponse | null;
  members: SquadMemberResponse[];
  onSquadChange: (value: AdminSquadResponse | null) => void;
  onMembersChange: (value: SquadMemberResponse[]) => void;
}) {
  const activeMembers = members.filter((member) => !member.isPendingInvite);
  const outgoingInvites = members.filter((member) => member.isPendingInvite);
  const [restrictionReason, setRestrictionReason] = useState("");
  const [isUpdatingRestriction, setIsUpdatingRestriction] = useState(false);
  const [squadName, setSquadName] = useState("");
  const [isEditingName, setIsEditingName] = useState(false);
  const [isUpdatingName, setIsUpdatingName] = useState(false);
  const [isDeletingSquad, setIsDeletingSquad] = useState(false);
  const [isDeletingAvatar, setIsDeletingAvatar] = useState(false);
  const [isUploadingAvatar, setIsUploadingAvatar] = useState(false);
  const [kickingMemberId, setKickingMemberId] = useState<string | null>(null);
  const [memberToKick, setMemberToKick] = useState<SquadMemberResponse | null>(null);
  const [isDeleteSquadModalOpen, setIsDeleteSquadModalOpen] = useState(false);
  const [isDeleteAvatarModalOpen, setIsDeleteAvatarModalOpen] = useState(false);
  const [avatarVersion, setAvatarVersion] = useState(() => Date.now());
  const avatarInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setSquadName(squad?.name ?? "");
  }, [squad?.id, squad?.name]);

  const deleteSquadAvatar = async () => {
    if (!squad || isDeletingAvatar) return;
    setIsDeletingAvatar(true);
    try {
      const updated = await deleteAdminSquadImage(token, squad.id);
      onSquadChange(updated);
      setAvatarVersion(Date.now());
      setIsDeleteAvatarModalOpen(false);
      toast.success("Аватарка сквада удалена.");
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось удалить аватарку сквада."));
    } finally {
      setIsDeletingAvatar(false);
    }
  };

  const uploadSquadAvatar = async (file: File) => {
    if (!squad || isUploadingAvatar) return;
    setIsUploadingAvatar(true);
    try {
      const updated = await uploadAdminSquadImage(token, squad.id, file);
      onSquadChange(updated);
      setAvatarVersion(Date.now());
      toast.success("Аватарка сквада обновлена.");
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось обновить аватарку сквада."));
    } finally {
      setIsUploadingAvatar(false);
      if (avatarInputRef.current) avatarInputRef.current.value = "";
    }
  };

  const kickMember = async (member: SquadMemberResponse) => {
    if (!squad || kickingMemberId || member.id === squad.leaderUserId) return;

    setKickingMemberId(member.id);
    try {
      await kickAdminSquadMember(token, squad.id, member.id);
      onMembersChange(members.filter((item) => item.id !== member.id));
      onSquadChange({
        ...squad,
        memberCount: Math.max(0, squad.memberCount - 1),
      });
      setMemberToKick(null);
      toast.success("Игрок кикнут из сквада.");
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось кикнуть игрока из сквада."));
    } finally {
      setKickingMemberId(null);
    }
  };

  const restrictSquad = async () => {
    if (!squad || isUpdatingRestriction) return;
    setIsUpdatingRestriction(true);
    try {
      const updated = await restrictAdminSquad(token, squad.id, {
        reason: restrictionReason.trim() || null,
      });
      onSquadChange(updated);
      toast.success("Ограничение на сквад выдано.");
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось выдать ограничение на сквад."));
    } finally {
      setIsUpdatingRestriction(false);
    }
  };

  const unrestrictSquad = async () => {
    if (!squad || isUpdatingRestriction) return;
    setIsUpdatingRestriction(true);
    try {
      const updated = await unrestrictAdminSquad(token, squad.id, {
        reason: restrictionReason.trim() || null,
      });
      onSquadChange(updated);
      toast.success("Ограничение со сквада снято.");
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось снять ограничение со сквада."));
    } finally {
      setIsUpdatingRestriction(false);
    }
  };

  const updateSquadName = async () => {
    if (!squad || isUpdatingName) return;
    const normalizedName = squadName.trim();
    if (!normalizedName) {
      toast.error("Укажите название сквада.");
      return;
    }
    if (normalizedName === squad.name) return;

    setIsUpdatingName(true);
    try {
      const updated = await patchAdminSquad(token, squad.id, { name: normalizedName });
      onSquadChange(updated);
      setSquadName(updated.name);
      setIsEditingName(false);
      toast.success("Название сквада обновлено.");
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось обновить название сквада."));
    } finally {
      setIsUpdatingName(false);
    }
  };

  const cancelSquadNameEdit = () => {
    setSquadName(squad?.name ?? "");
    setIsEditingName(false);
  };

  const removeSquad = async () => {
    if (!squad || isDeletingSquad) return;
    setIsDeletingSquad(true);
    try {
      await deleteAdminSquad(token, squad.id);
      setIsDeleteSquadModalOpen(false);
      toast.success("Сквад удален.");
      pushUrl(`${paths.admin}?tab=squads`);
    } catch (cause) {
      toast.error(toDisplayError(cause, "Не удалось удалить сквад."));
    } finally {
      setIsDeletingSquad(false);
    }
  };

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <AdminLink className="btn btn-sm" href={`${paths.admin}?tab=squads`}>
          ← К сквадам
        </AdminLink>
      </section>

      <section className="card admin-card">
        {!squad ? (
          <LoadingState title="Загружаем профиль сквада" />
        ) : (
          <>
            <div className="admin-squad-head admin-detail-hero">
              {squad.imageUrl ? (
                <img
                  src={appendVersion(squad.imageUrl, avatarVersion)}
                  alt={squad.name}
                  className="admin-avatar admin-avatar-lg"
                />
              ) : (
                <span className="ui-avatar">{initials(squad.name)}</span>
              )}
              <div className="admin-row-user-text">
                {!isEditingName ? (
                  <div className="admin-squad-name-row">
                    <h2 className="card-title">{squad.name}</h2>
                    <button
                      type="button"
                      className="admin-icon-button"
                      aria-label="Изменить название сквада"
                      disabled={isUpdatingName || isDeletingSquad}
                      onClick={() => setIsEditingName(true)}
                    >
                      <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      >
                        <path d="M12 20h9" />
                        <path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4 12.5-12.5z" />
                      </svg>
                    </button>
                  </div>
                ) : (
                  <div className="admin-squad-name-edit">
                    <input
                      className="ui-input"
                      value={squadName}
                      onChange={(event) => setSquadName(event.target.value)}
                      placeholder="Название сквада"
                      autoFocus
                    />
                    <div className="admin-squad-name-edit-actions">
                      <button
                        type="button"
                        className="btn btn-sm"
                        disabled={
                          isUpdatingName || !squadName.trim() || squadName.trim() === squad.name
                        }
                        onClick={() => void updateSquadName()}
                      >
                        {isUpdatingName ? "Сохраняем..." : "Сохранить"}
                      </button>
                      <button
                        type="button"
                        className="btn btn-sm"
                        disabled={isUpdatingName}
                        onClick={cancelSquadNameEdit}
                      >
                        Отмена
                      </button>
                    </div>
                  </div>
                )}
                <small>{squad.id}</small>
              </div>
              <div className="admin-squad-actions">
                <input
                  ref={avatarInputRef}
                  type="file"
                  accept="image/*"
                  className="admin-hidden-file-input"
                  onChange={(event) => {
                    const file = event.target.files?.[0];
                    if (file) void uploadSquadAvatar(file);
                  }}
                />
                <button
                  className="btn"
                  type="button"
                  disabled={isUploadingAvatar || isDeletingAvatar}
                  onClick={() => avatarInputRef.current?.click()}
                >
                  {isUploadingAvatar ? "Загружаем..." : "Сменить аватарку сквада"}
                </button>
                <button
                  className="btn danger"
                  type="button"
                  disabled={!squad.imageUrl || isDeletingAvatar}
                  onClick={() => setIsDeleteAvatarModalOpen(true)}
                >
                  Удалить аватарку сквада
                </button>
                <button
                  type="button"
                  className="btn danger"
                  disabled={isDeletingSquad}
                  onClick={() => setIsDeleteSquadModalOpen(true)}
                >
                  {isDeletingSquad ? "Удаляем..." : "Удалить сквад"}
                </button>
              </div>
            </div>

            <dl className="admin-kv">
              <div>
                <dt>Создан</dt>
                <dd>{formatDateTime(squad.createdAt).replace(", ", " ")}</dd>
              </div>
              <div>
                <dt>Обновлён</dt>
                <dd>{formatDateTime(squad.updatedAt).replace(", ", " ")}</dd>
              </div>
              <div>
                <dt>Лидер</dt>
                <dd>{squad.leaderUserId}</dd>
              </div>
            </dl>

            <section className="admin-card-subsection">
              <h3 className="card-title">Ограничение сквада</h3>
              <div className="admin-restriction-status">
                <span
                  className={`ui-badge ${squad.isRestricted ? "ui-badge-warning" : "ui-badge-success"}`}
                >
                  {squad.isRestricted ? "Сквад ограничен" : "Ограничений нет"}
                </span>
                {squad.restrictionReason ? <small>{squad.restrictionReason}</small> : null}
              </div>
              <div className="admin-restriction-toolbar">
                <input
                  className="ui-input"
                  value={restrictionReason}
                  onChange={(event) => setRestrictionReason(event.target.value)}
                  placeholder="Причина ограничения"
                />
                <button
                  type="button"
                  className="btn btn-sm danger"
                  disabled={isUpdatingRestriction || squad.isRestricted}
                  onClick={() => void restrictSquad()}
                >
                  Выдать ограничение
                </button>
                <button
                  type="button"
                  className="btn btn-sm"
                  disabled={isUpdatingRestriction || !squad.isRestricted}
                  onClick={() => void unrestrictSquad()}
                >
                  Снять ограничение
                </button>
              </div>
            </section>

            <div className="admin-squad-columns">
              <section>
                <h3>Участники</h3>
                <ul className="admin-member-list">
                  {activeMembers.map((member) => (
                    <li key={member.id}>
                      <AdminLink className="admin-member-card-main" href={adminUserPath(member.id)}>
                        {member.avatarUrl ? (
                          <img
                            src={member.avatarUrl}
                            alt={member.username}
                            className="admin-avatar"
                          />
                        ) : (
                          <span className="ui-avatar ui-avatar-sm">
                            {initials(member.username)}
                          </span>
                        )}
                        <span className="admin-member-name">{member.username}</span>
                      </AdminLink>
                      {member.id === squad.leaderUserId && (
                        <span className="ui-badge ui-badge-secondary">Лидер</span>
                      )}
                      {member.id !== squad.leaderUserId ? (
                        <button
                          type="button"
                          className="btn btn-sm danger admin-member-kick-button"
                          disabled={kickingMemberId !== null}
                          onClick={() => setMemberToKick(member)}
                        >
                          {kickingMemberId === member.id ? "Кикаем..." : "Кикнуть"}
                        </button>
                      ) : null}
                    </li>
                  ))}
                </ul>
              </section>

              <section>
                <h3>Исходящие запросы</h3>
                <ul className="admin-member-list">
                  {outgoingInvites.map((member) => (
                    <li key={member.inviteId ?? member.id}>
                      <AdminLink className="admin-member-card-main" href={adminUserPath(member.id)}>
                        {member.avatarUrl ? (
                          <img
                            src={member.avatarUrl}
                            alt={member.username}
                            className="admin-avatar"
                          />
                        ) : (
                          <span className="ui-avatar ui-avatar-sm">
                            {initials(member.username)}
                          </span>
                        )}
                        <span className="admin-member-name">{member.username}</span>
                      </AdminLink>
                      <span className="ui-badge ui-badge-warning">Invite</span>
                    </li>
                  ))}
                  {outgoingInvites.length === 0 ? (
                    <li className="admin-inline-muted">Нет исходящих инвайтов.</li>
                  ) : null}
                </ul>
              </section>
            </div>
          </>
        )}
      </section>

      {squad && isDeleteAvatarModalOpen ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => !isDeletingAvatar && setIsDeleteAvatarModalOpen(false)}
          >
            <div
              className="ui-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="delete-squad-avatar-modal-title"
              onClick={(event) => event.stopPropagation()}
            >
              <div className="ui-modal-header">
                <h2 id="delete-squad-avatar-modal-title" className="ui-modal-title">
                  Удалить аватарку сквада?
                </h2>
                <button
                  className="ui-modal-close"
                  type="button"
                  aria-label="Закрыть"
                  onClick={() => setIsDeleteAvatarModalOpen(false)}
                  disabled={isDeletingAvatar}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>
                  Аватарка сквада <strong>{squad.name}</strong> будет удалена. Подтвердите действие.
                </p>
              </div>
              <div className="ui-modal-footer">
                <button
                  className="btn"
                  type="button"
                  onClick={() => setIsDeleteAvatarModalOpen(false)}
                  disabled={isDeletingAvatar}
                >
                  Отменить
                </button>
                <button
                  className="btn danger"
                  type="button"
                  onClick={() => void deleteSquadAvatar()}
                  disabled={isDeletingAvatar}
                >
                  {isDeletingAvatar ? "Удаление..." : "Удалить"}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}

      {squad && memberToKick ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => !kickingMemberId && setMemberToKick(null)}
          >
            <div
              className="ui-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="kick-squad-member-modal-title"
              onClick={(event) => event.stopPropagation()}
            >
              <div className="ui-modal-header">
                <h2 id="kick-squad-member-modal-title" className="ui-modal-title">
                  Кикнуть участника?
                </h2>
                <button
                  className="ui-modal-close"
                  type="button"
                  aria-label="Закрыть"
                  onClick={() => setMemberToKick(null)}
                  disabled={Boolean(kickingMemberId)}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>
                  Игрок <strong>{memberToKick.username}</strong> будет удален из сквада{" "}
                  <strong>{squad.name}</strong>. Приглашение или вступление придется оформить
                  заново.
                </p>
              </div>
              <div className="ui-modal-footer">
                <button
                  className="btn"
                  type="button"
                  onClick={() => setMemberToKick(null)}
                  disabled={Boolean(kickingMemberId)}
                >
                  Отмена
                </button>
                <button
                  className="btn danger"
                  type="button"
                  onClick={() => void kickMember(memberToKick)}
                  disabled={Boolean(kickingMemberId)}
                >
                  {kickingMemberId ? "Кикаем..." : "Кикнуть"}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}

      {squad && isDeleteSquadModalOpen ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => !isDeletingSquad && setIsDeleteSquadModalOpen(false)}
          >
            <div
              className="ui-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="delete-squad-modal-title"
              onClick={(event) => event.stopPropagation()}
            >
              <div className="ui-modal-header">
                <h2 id="delete-squad-modal-title" className="ui-modal-title">
                  Удалить сквад?
                </h2>
                <button
                  className="ui-modal-close"
                  type="button"
                  aria-label="Закрыть"
                  onClick={() => setIsDeleteSquadModalOpen(false)}
                  disabled={isDeletingSquad}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>
                  Сквад <strong>{squad.name}</strong> будет удален без возможности восстановления.
                  Подтвердите действие.
                </p>
              </div>
              <div className="ui-modal-footer">
                <button
                  className="btn"
                  type="button"
                  onClick={() => setIsDeleteSquadModalOpen(false)}
                  disabled={isDeletingSquad}
                >
                  Отменить
                </button>
                <button
                  className="btn danger"
                  type="button"
                  onClick={() => void removeSquad()}
                  disabled={isDeletingSquad}
                >
                  {isDeletingSquad ? "Удаление..." : "Удалить"}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </div>
  );
}
