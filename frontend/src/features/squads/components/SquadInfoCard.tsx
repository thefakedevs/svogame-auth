import { useEffect, useState } from "react";
import toast from "react-hot-toast";
import { deleteSquad, patchSquad, uploadSquadImage } from "../../../api/squads";
import { toDisplayError } from "../../../api/http";
import AppPortal from "../../../shared/ui/portal/AppPortal";
import { formatDate, formatImageLimit } from "../lib/format";
import {
  hasInvalidSquadNameBoundary,
  hasInvalidSquadNameByConfig,
  isInvalidSquadNameLength,
  sanitizeSquadName,
  validateSquadImageByConfig,
} from "../lib/validation";
import type { SquadSectionProps } from "../types";

type Props = SquadSectionProps & {
  isLeader: boolean;
};

export default function SquadInfoCard({ data, isLeader, authToken, onChanged }: Props) {
  const [isUploadingImage, setIsUploadingImage] = useState(false);
  const [isDragActive, setIsDragActive] = useState(false);
  const [nameDraft, setNameDraft] = useState(data.squad?.name ?? "");
  const [hasInvalidNameInput, setHasInvalidNameInput] = useState(false);
  const [isEditingName, setIsEditingName] = useState(false);
  const [isSavingName, setIsSavingName] = useState(false);
  const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false);
  const [isDeletingSquad, setIsDeletingSquad] = useState(false);
  const squadImageUrl = data.squad?.imageUrl
    ? `${data.squad.imageUrl}?v=${encodeURIComponent(data.squad.updatedAt)}`
    : null;
  const isAvatarUploadDisabled = !isLeader || isUploadingImage;
  const squadAvatarInputId = `squad-avatar-upload-${data.squad?.id ?? "current"}`;
  const squadImageHint = `до ${data.squadConfig.imageMaxWidth}x${data.squadConfig.imageMaxHeight} и ${formatImageLimit(data.squadConfig.imageMaxBytes)}`;
  const avatarActionLabel = isUploadingImage
    ? "Загрузка..."
    : isLeader
      ? `Аватар ${squadImageHint}`
      : "Недоступно: только лидер";

  useEffect(() => {
    setNameDraft(data.squad?.name ?? "");
    setHasInvalidNameInput(false);
  }, [data.squad?.name]);

  const onNameDraftChange = (value: string) => {
    const sanitized = sanitizeSquadName(value);
    const trimmed = sanitized.trim();

    setNameDraft(sanitized);
    setHasInvalidNameInput(
      sanitized !== value ||
        hasInvalidSquadNameBoundary(trimmed) ||
        isInvalidSquadNameLength(trimmed) ||
        hasInvalidSquadNameByConfig(trimmed, data.squadConfig),
    );
  };

  const onImageSelected = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file || !data.squad) return;

    const imageValidationError = await validateSquadImageByConfig(file, data.squadConfig);
    if (imageValidationError) {
      toast.error(imageValidationError);
      event.target.value = "";
      return;
    }

    setIsUploadingImage(true);
    const request = uploadSquadImage(authToken, data.squad.id, file);
    toast.promise(request, {
      loading: "Загружаем аватар сквада...",
      success: "Аватар сквада обновлен.",
      error: (cause) => toDisplayError(cause, "Не удалось загрузить аватар сквада."),
    });

    try {
      await request;
      await onChanged();
    } finally {
      setIsUploadingImage(false);
      setIsDragActive(false);
      event.target.value = "";
    }
  };

  const onSaveName = async () => {
    if (!data.squad) return;
    const trimmed = nameDraft.trim();
    if (hasInvalidSquadNameByConfig(trimmed, data.squadConfig)) {
      setHasInvalidNameInput(true);
      return;
    }
    if (!trimmed || trimmed === data.squad.name) {
      setIsEditingName(false);
      setNameDraft(data.squad.name);
      return;
    }

    setIsSavingName(true);
    const request = patchSquad(authToken, data.squad.id, trimmed);
    toast.promise(request, {
      loading: "Сохраняем сквад...",
      success: "Настройки сквада обновлены.",
      error: (cause) => toDisplayError(cause, "Не удалось обновить сквад."),
    });

    try {
      await request;
      await onChanged();
      setIsEditingName(false);
    } finally {
      setIsSavingName(false);
    }
  };

  const onCancelNameEdit = () => {
    setNameDraft(data.squad?.name ?? "");
    setHasInvalidNameInput(false);
    setIsEditingName(false);
  };

  const onDeleteSquad = async () => {
    if (!data.squad) return;

    setIsDeletingSquad(true);
    const request = deleteSquad(authToken, data.squad.id);
    toast.promise(request, {
      loading: "Распускаем сквад...",
      success: "Сквад удален.",
      error: (cause) => toDisplayError(cause, "Не удалось удалить сквад."),
    });

    try {
      await request;
      setIsDeleteModalOpen(false);
      await onChanged();
    } finally {
      setIsDeletingSquad(false);
    }
  };

  return (
    <section className="card profile-panel">
      <div className="ui-card-header">
        <h2 className="card-title">{isLeader ? "Настройки сквада" : "Информация о скваде"}</h2>
      </div>
      <div className="profile-stack">
        <div className="profile-squad-head-row">
          {isLeader ? (
            <label
              className={`profile-squad-avatar-tile is-clickable ${isDragActive ? "is-drag-active" : ""}`}
              aria-disabled={isAvatarUploadDisabled}
              aria-label={avatarActionLabel}
              htmlFor={squadAvatarInputId}
              onDragEnter={(event) => {
                if (!event.dataTransfer.types.includes("Files")) return;
                setIsDragActive(true);
              }}
              onDragOver={(event) => {
                if (!event.dataTransfer.types.includes("Files")) return;
                event.preventDefault();
                event.dataTransfer.dropEffect = "copy";
                if (!isDragActive) setIsDragActive(true);
              }}
              onDragLeave={(event) => {
                const nextTarget = event.relatedTarget;
                if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) return;
                setIsDragActive(false);
              }}
              onDrop={() => {
                setIsDragActive(false);
              }}
            >
              {squadImageUrl ? (
                <img
                  className="profile-squad-avatar-image"
                  src={squadImageUrl}
                  alt={`Аватар сквада ${data.squad?.name ?? ""}`}
                />
              ) : (
                <span className="profile-squad-avatar-placeholder" aria-hidden>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                  <span className="profile-squad-avatar-placeholder-copy">{squadImageHint}</span>
                </span>
              )}
              <span className="profile-squad-avatar-tooltip" role="tooltip">
                {avatarActionLabel}
              </span>
              <span className="profile-squad-avatar-overlay" aria-hidden>
                Отпустите файл
              </span>
              <input
                id={squadAvatarInputId}
                className="profile-squad-avatar-input"
                type="file"
                accept="image/*"
                disabled={isUploadingImage}
                onChange={(event) => void onImageSelected(event)}
              />
            </label>
          ) : (
            <div
              className="profile-squad-avatar-tile is-readonly"
              aria-disabled="true"
              aria-label={avatarActionLabel}
            >
              {squadImageUrl ? (
                <img
                  className="profile-squad-avatar-image"
                  src={squadImageUrl}
                  alt={`Аватар сквада ${data.squad?.name ?? ""}`}
                />
              ) : (
                <span className="profile-squad-avatar-placeholder" aria-hidden>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                  <span className="profile-squad-avatar-placeholder-copy">Недоступно</span>
                </span>
              )}
              <span className="profile-squad-avatar-tooltip" role="tooltip">
                {avatarActionLabel}
              </span>
            </div>
          )}
          <div className="profile-squad-name-block">
            {!isEditingName ? (
              <div className="profile-squad-name-row">
                <strong className="profile-squad-name-inline">
                  {data.squad?.name ?? "Нет данных"}
                </strong>
                {isLeader ? (
                  <button
                    className="profile-icon-button"
                    type="button"
                    aria-label="Изменить название сквада"
                    onClick={() => setIsEditingName(true)}
                    disabled={isSavingName || isDeletingSquad}
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
                ) : null}
              </div>
            ) : null}
            {isLeader && isEditingName ? (
              <div className="profile-form">
                <div className={`ui-field ${hasInvalidNameInput ? "ui-field-error" : ""}`}>
                  <input
                    id="squad-name"
                    className="ui-input"
                    value={nameDraft}
                    onChange={(event) => onNameDraftChange(event.target.value)}
                    placeholder="Введите название сквада"
                    minLength={data.squadConfig.nameMinChars}
                    maxLength={data.squadConfig.nameMaxChars}
                    inputMode="text"
                    autoComplete="off"
                    autoFocus
                  />
                  <div className={`ui-hint ${hasInvalidNameInput ? "ui-hint-error" : ""}`}>
                    Лимиты: {data.squadConfig.nameMinChars}-{data.squadConfig.nameMaxChars}{" "}
                    символов. Разрешены: латиница, кириллица и `-`. Имя не может начинаться или
                    заканчиваться на `-`.
                  </div>
                </div>
                <div className="profile-actions">
                  <button
                    className="btn primary"
                    type="button"
                    disabled={isSavingName}
                    onClick={() => void onSaveName()}
                  >
                    {isSavingName ? "Сохранение..." : "Сохранить"}
                  </button>
                  <button
                    className="btn"
                    type="button"
                    disabled={isSavingName}
                    onClick={onCancelNameEdit}
                  >
                    Отменить
                  </button>
                </div>
              </div>
            ) : null}
          </div>
        </div>
        <div className="profile-inline-card">
          <strong>Создан</strong>
          <span className="profile-subtle">
            {formatDate(data.squad?.createdAt).replace(", ", " ")}
          </span>
        </div>
        <div className="profile-inline-card">
          <strong>Участников</strong>
          <span className="profile-subtle">
            {data.squad?.memberCount ?? 0} / {data.squad?.maxMembers ?? data.squadConfig.maxMembers}
          </span>
        </div>
        <div className="profile-inline-card">
          <strong>Лидер</strong>
          <span className="profile-subtle">
            {data.squadMembers.find((member) => member.isLeader)?.username ?? "Нет данных"}
          </span>
        </div>
        {isLeader ? (
          <div className="profile-actions">
            <button
              className="btn danger"
              type="button"
              disabled={isDeletingSquad}
              onClick={() => setIsDeleteModalOpen(true)}
            >
              {isDeletingSquad ? "Удаление..." : "Удалить сквад"}
            </button>
          </div>
        ) : null}
      </div>

      {isLeader && isDeleteModalOpen ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => !isDeletingSquad && setIsDeleteModalOpen(false)}
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
                  onClick={() => setIsDeleteModalOpen(false)}
                  disabled={isDeletingSquad}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                <p>
                  Сквад <strong>{data.squad?.name ?? ""}</strong> будет удален без возможности
                  восстановления. Подтвердите действие.
                </p>
              </div>
              <div className="ui-modal-footer">
                <button
                  className="btn"
                  type="button"
                  onClick={() => setIsDeleteModalOpen(false)}
                  disabled={isDeletingSquad}
                >
                  Отменить
                </button>
                <button
                  className="btn danger"
                  type="button"
                  onClick={() => void onDeleteSquad()}
                  disabled={isDeletingSquad}
                >
                  {isDeletingSquad ? "Удаление..." : "Удалить"}
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </section>
  );
}
