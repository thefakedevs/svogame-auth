import "../../pages/UiKitPage.css";
import "../../ui/ui.css";
import type { PowProgressUpdate } from "../../services/pow.ts";

export type AuthFlowStageState = {
  status: "loading" | "solving_pow" | "redirecting" | "error";
  oauthUrl?: string;
  errorMessage?: string;
  powProgress?: PowProgressUpdate;
  powComplexity?: number;
};

function formatExpectedTime(progress: PowProgressUpdate | undefined): string {
  if (!progress) return "подсчет...";
  const { eta } = progress;
  if (!Number.isFinite(eta) || eta < 0) return "—";
  if (eta === 0) return "~0 c";
  return `~${eta} c`;
}

function formatHashRate(hps: number): string {
  if (!Number.isFinite(hps) || hps <= 0) return "0 kH/s";
  const k = hps / 1000;
  if (k < 10) return `${k.toFixed(1)} kH/s`;
  return `${Math.round(k)} kH/s`;
}

function AuthSpinner() {
  return (
    <span className="ui-spinner ui-spinner-lg" role="status" aria-label="Загрузка">
      <span className="ui-spinner-track" aria-hidden>
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
        <span className="ui-spinner-orb" />
      </span>
    </span>
  );
}

function PowCheckLockIcon() {
  return (
    <div className="ui-pow-check-icon" aria-hidden>
      <svg
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        stroke="currentColor"
        strokeWidth="1.5"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z"
        />
      </svg>
    </div>
  );
}

function DiscordMarkIcon() {
  return (
    <div className="ui-auth-flow-icon ui-auth-flow-icon--discord" aria-hidden>
      <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.086-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.086-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
      </svg>
    </div>
  );
}

function ErrorHeroIcon() {
  return (
    <div className="ui-auth-flow-icon ui-auth-flow-icon--error" aria-hidden>
      <svg
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        stroke="currentColor"
        strokeWidth="1.5"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 17.25h.007v.008H12v-.008z"
        />
      </svg>
    </div>
  );
}

function renderHero(status: AuthFlowStageState["status"]) {
  switch (status) {
    case "loading":
      return <AuthSpinner />;
    case "solving_pow":
      return <PowCheckLockIcon />;
    case "redirecting":
      return <DiscordMarkIcon />;
    case "error":
      return <ErrorHeroIcon />;
    default:
      return null;
  }
}

function stageAriaLabel(status: AuthFlowStageState["status"]): string {
  switch (status) {
    case "loading":
      return "Подготовка авторизации";
    case "solving_pow":
      return "Проверка на бота, пожалуйста подождите";
    case "redirecting":
      return "Переход в Discord";
    case "error":
      return "Ошибка авторизации";
    default:
      return "Авторизация";
  }
}

export interface AuthFlowStagesProps {
  stage: AuthFlowStageState;
  onRetryError: () => void;
}

export default function AuthFlowStages({ stage, onRetryError }: AuthFlowStagesProps) {
  const { status } = stage;
  const busy = status === "loading" || status === "solving_pow";
  const redirectUrl = status === "redirecting" ? (stage.oauthUrl ?? "") : "";

  const title =
    status === "loading"
      ? "Подключение к Discord"
      : status === "solving_pow"
        ? "Проверка на бота"
        : status === "redirecting"
          ? "Переход в Discord"
          : "Ошибка авторизации";

  const lead =
    status === "loading"
      ? "Готовим вход: получаем параметры авторизации и защиту для сессии."
      : status === "solving_pow"
        ? "Короткая локальная проверка устройства. Она помогает защищать сервис от ботов и перегрузки."
        : status === "redirecting"
          ? "Сейчас откроется Discord. Если переход не случился автоматически, нажмите кнопку ниже."
          : (stage.errorMessage ?? "Не удалось инициализировать авторизацию.");

  const percents =
    status === "solving_pow" && stage.powProgress
      ? Math.min(100, Math.max(0, stage.powProgress.percents))
      : 0;
  const showStallHint =
    status === "solving_pow" && stage.powProgress !== undefined && percents >= 100;
  const hashRate = status === "solving_pow" ? (stage.powProgress?.hashRatePerSec ?? 0) : 0;
  const powComplexity = status === "solving_pow" ? (stage.powComplexity ?? 0) : 0;

  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div
        className="ui-kit-page ui-auth-flow ui-pow-check vt-auth-root"
        data-phase={status}
        role="status"
        aria-live="polite"
        aria-busy={busy}
        aria-label={stageAriaLabel(status)}
      >
        <div key={status} className="ui-pow-check-stack vt-auth-stack">
          <div className="vt-slot-hero ui-auth-flow-hero">{renderHero(status)}</div>

          <h1 className="ui-pow-check-title">{title}</h1>

          <p
            className={
              status === "error"
                ? "ui-pow-check-lead ui-auth-flow-lead--error"
                : "ui-pow-check-lead"
            }
          >
            {lead}
          </p>

          <div className="vt-slot-main">
            {status === "solving_pow" && (
              <>
                <div className="ui-pow-check-bar">
                  <progress value={percents} max={100} />
                </div>
                <p className="ui-pow-check-tech">
                  Сложность: {powComplexity}
                  <span className="ui-pow-check-tech-sep" aria-hidden>
                    ·
                  </span>
                  Скорость: {formatHashRate(hashRate)}
                </p>
                <p className="ui-pow-check-eta">
                  Ожидаемое время: {formatExpectedTime(stage.powProgress)}
                </p>
                {showStallHint && (
                  <p className="ui-pow-check-hint">Дольше обычного, но проверка почти завершена</p>
                )}
              </>
            )}
          </div>

          <div className="vt-slot-actions">
            {status === "redirecting" && redirectUrl && (
              <button
                className="btn primary"
                type="button"
                onClick={() => (window.location.href = redirectUrl)}
              >
                Открыть Discord
              </button>
            )}
            {status === "error" && (
              <button className="btn primary" type="button" onClick={onRetryError}>
                Попробовать еще раз
              </button>
            )}
          </div>
        </div>
      </div>
    </>
  );
}
