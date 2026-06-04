import { useEffect, useMemo, useState } from "react";
import { getAdminUser, type AdminUserResponse } from "../../api/admin";
import {
  getAdminLittlemiceBinaryContent,
  getAdminLittlemiceCheck,
  getAdminLittlemiceTextContent,
  listAdminUserLittlemiceChecks,
  type LittlemiceCheckDetailResponse,
  type LittlemiceCheckListItemResponse,
  type LittlemiceCheckListResponse,
} from "../../api/littlemice";
import { toDisplayError } from "../../api/http";
import {
  adminUserLittlemiceCheckPath,
  adminUserLittlemicePath,
  adminUserPath,
} from "../../routes/paths";
import AppPortal from "../../shared/ui/portal/AppPortal";
import ErrorState from "../ErrorState";
import LoadingState from "../LoadingState";
import AdminLink from "./AdminLink";

const PAGE_SIZE = 20;

const dateTimeFormatter = new Intl.DateTimeFormat("ru-RU", {
  day: "2-digit",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

function formatDateTime(value?: string | null) {
  if (!value) return "—";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(", ", " ");
}

function formatBytes(value?: number | null) {
  if (value == null) return "—";
  if (value < 1024) return `${value} Б`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`;
  return `${(value / 1024 / 1024).toFixed(1)} МБ`;
}

function downloadText(filename: string, text: string, type = "text/plain;charset=utf-8") {
  const objectUrl = URL.createObjectURL(new Blob([text], { type }));
  const link = document.createElement("a");
  link.href = objectUrl;
  link.download = filename;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
}

function statusLabel(status: string) {
  if (status === "pending") return "Ожидает";
  if (status === "passed") return "Пройдена";
  if (status === "failed_timeout") return "Таймаут";
  if (status === "failed_client_error") return "Ошибка клиента";
  return status;
}

function statusClassName(status: string) {
  if (status === "passed") return "ui-badge ui-badge-success";
  if (status === "pending") return "ui-badge ui-badge-secondary";
  if (status.startsWith("failed")) return "ui-badge ui-badge-warning";
  return "ui-badge ui-badge-neutral";
}

function statusTone(status: string) {
  if (status === "passed") return "success";
  if (status === "pending") return "pending";
  if (status.startsWith("failed")) return "failed";
  return "neutral";
}

function finishTime(item: LittlemiceCheckListItemResponse) {
  return item.completedAt ?? item.receivedAt ?? null;
}

function formatShortId(value: string) {
  return value.length > 14 ? `${value.slice(0, 8)}...${value.slice(-4)}` : value;
}

function playerLabel(user: AdminUserResponse | null | undefined, playerUuid: string) {
  if (user) return user.username;
  if (user === null) return formatShortId(playerUuid);
  return "Загрузка...";
}

function getPaginationPages(page: number, totalPages: number) {
  const pages = new Set(
    [1, totalPages, page - 1, page, page + 1].filter((item) => item >= 1 && item <= totalPages),
  );
  return Array.from(pages).sort((left, right) => left - right);
}

function LittlemicePagination({
  page,
  totalPages,
  onPageChange,
}: {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
}) {
  const pages = getPaginationPages(page, totalPages);

  return (
    <nav className="admin-pagination" aria-label="Страницы проверок littlemice">
      <ul className="ui-pagination">
        <li>
          <button
            type="button"
            className="ui-pagination-btn"
            aria-label="Предыдущая страница"
            disabled={page <= 1}
            onClick={() => onPageChange(Math.max(1, page - 1))}
          >
            ‹
          </button>
        </li>
        {pages.map((item, index) => (
          <li key={item}>
            {index > 0 && item - pages[index - 1] > 1 ? (
              <span className="ui-pagination-ellipsis">…</span>
            ) : null}
            <button
              type="button"
              className="ui-pagination-btn"
              aria-label={`Страница ${item}`}
              aria-current={page === item ? "page" : undefined}
              onClick={() => onPageChange(item)}
            >
              {item}
            </button>
          </li>
        ))}
        <li>
          <button
            type="button"
            className="ui-pagination-btn"
            aria-label="Следующая страница"
            disabled={page >= totalPages}
            onClick={() => onPageChange(Math.min(totalPages, page + 1))}
          >
            ›
          </button>
        </li>
      </ul>
    </nav>
  );
}

function LittlemiceCheckSummary({
  item,
  userId,
}: {
  item: LittlemiceCheckListItemResponse;
  userId: string;
}) {
  return (
    <AdminLink
      href={adminUserLittlemiceCheckPath(userId, item.id)}
      className={`admin-littlemice-timeline-item admin-littlemice-timeline-item--${statusTone(item.status)}`}
    >
      <span className="admin-littlemice-timeline-point" aria-hidden="true" />
      <span className="admin-littlemice-timeline-main">
        <span className="admin-littlemice-timeline-head">
          <strong>{formatDateTime(item.requestedAt)}</strong>
          <span className={statusClassName(item.status)}>{statusLabel(item.status)}</span>
        </span>
        <span className="admin-littlemice-timeline-meta">
          <span title={item.id}>Check: {formatShortId(item.id)}</span>
          <span>Завершение: {formatDateTime(finishTime(item))}</span>
        </span>
        {item.failureReason ? (
          <span className="admin-littlemice-failure">Причина: {item.failureReason}</span>
        ) : null}
      </span>
    </AdminLink>
  );
}

function AdminUserLittlemiceListView({ token, userId }: { token: string; userId: string }) {
  const [page, setPage] = useState(1);
  const [response, setResponse] = useState<LittlemiceCheckListResponse | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      setResponse(null);
      setError("");
      try {
        const loaded = await listAdminUserLittlemiceChecks(token, userId, {
          page,
          perPage: PAGE_SIZE,
        });
        if (!cancelled) setResponse(loaded);
      } catch (cause) {
        if (!cancelled)
          setError(toDisplayError(cause, "Не удалось загрузить проверки littlemice."));
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [page, token, userId]);

  const totalPages = Math.max(1, response?.totalPages ?? 1);

  return (
    <div className="admin-page">
      <section className="admin-top-actions admin-page-toolbar">
        <AdminLink className="btn btn-sm" href={adminUserPath(userId)}>
          ← К профилю игрока
        </AdminLink>
      </section>

      <section className="card admin-card">
        <div className="admin-littlemice-head">
          <div>
            <h2 className="card-title">Проверки littlemice</h2>
            <p className="card-text">
              История проверок конкретного игрока. Открывайте карточку, чтобы посмотреть скриншоты,
              client info и лог.
            </p>
            <p className="admin-inline-muted">
              User ID: <code>{userId}</code>
            </p>
          </div>
          {response ? (
            <div className="admin-littlemice-summary-badges">
              <span className="ui-badge ui-badge-neutral">Всего: {response.total}</span>
              <span className="ui-badge ui-badge-neutral">
                Страница {page} из {totalPages}
              </span>
            </div>
          ) : null}
        </div>

        {!response && !error ? <LoadingState title="Загружаем проверки" /> : null}
        {error ? <ErrorState title="Проверки недоступны" message={error} /> : null}
        {response ? (
          response.items.length > 0 ? (
            <>
              <div className="admin-littlemice-timeline">
                {response.items.map((item) => (
                  <LittlemiceCheckSummary key={item.id} item={item} userId={userId} />
                ))}
              </div>
              {totalPages > 1 ? (
                <LittlemicePagination page={page} totalPages={totalPages} onPageChange={setPage} />
              ) : null}
            </>
          ) : (
            <p className="admin-inline-muted">У игрока еще нет проверок littlemice.</p>
          )
        ) : null}
      </section>
    </div>
  );
}

type ClientInfoResourcePack = {
  id?: unknown;
  n?: unknown;
  d?: unknown;
  r?: unknown;
  loaded?: unknown;
  h?: unknown;
  hw?: unknown;
};

type ClientInfoShaderPack = {
  name?: unknown;
  loaded?: unknown;
};

type ClientInfoMod = {
  id?: unknown;
  name?: unknown;
  version?: unknown;
  namespace?: unknown;
};

type ClientInfoRuntimeOddNative = {
  n?: unknown;
  p?: unknown;
  r?: unknown;
};

type ClientInfoRuntime = {
  pid?: unknown;
  vm?: unknown;
  agentArgs?: unknown[];
  attach?: {
    api?: unknown;
    self?: unknown;
    disabled?: unknown;
    jmx?: unknown;
  };
  native?: {
    ok?: unknown;
    count?: unknown;
    odd?: ClientInfoRuntimeOddNative[];
  };
};

type ParsedClientInfo = {
  t?: unknown;
  mc?: unknown;
  vt?: unknown;
  pn?: unknown;
  pu?: unknown;
  srv?: unknown;
  rp?: {
    items?: ClientInfoResourcePack[];
  };
  sp?: {
    active?: unknown;
    items?: ClientInfoShaderPack[];
  };
  mods?: ClientInfoMod[];
  rt?: ClientInfoRuntime;
};

function asText(value: unknown) {
  if (typeof value === "string") return value || "—";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return "—";
}

function parseClientInfo(text: string): ParsedClientInfo | null {
  try {
    const parsed = JSON.parse(text) as unknown;
    return parsed && typeof parsed === "object" ? (parsed as ParsedClientInfo) : null;
  } catch {
    return null;
  }
}

function RuntimeDiagnosticsView({ runtime }: { runtime?: ClientInfoRuntime }) {
  if (!runtime) return null;

  const agentArgs = Array.isArray(runtime.agentArgs) ? runtime.agentArgs : [];
  const oddNative = Array.isArray(runtime.native?.odd) ? runtime.native.odd : [];
  const attachApiAvailable = runtime.attach?.api === true && runtime.attach?.disabled === false;
  const attachSelfEnabled = runtime.attach?.self === "true";
  const jmxEnabled = runtime.attach?.jmx != null;

  const risks = [
    agentArgs.length > 0 ? { level: "high", label: "JVM agent args" } : null,
    attachApiAvailable ? { level: "medium", label: "Attach API доступен" } : null,
    attachSelfEnabled ? { level: "medium", label: "Attach self=true" } : null,
    jmxEnabled ? { level: "medium", label: "JMX flag" } : null,
    oddNative.length > 0 ? { level: "high", label: "Нетипичные DLL" } : null,
  ].filter(Boolean) as Array<{ level: "high" | "medium"; label: string }>;

  return (
    <section className="admin-littlemice-client-section">
      <div className="admin-littlemice-section-head">
        <h3>Runtime / Injection diagnostics</h3>
        <div className="admin-littlemice-risk-list">
          {risks.length ? (
            risks.map((risk) => (
              <span
                key={risk.label}
                className={`ui-badge ${risk.level === "high" ? "ui-badge-warning" : "ui-badge-secondary"}`}
              >
                {risk.label}
              </span>
            ))
          ) : (
            <span className="ui-badge ui-badge-neutral">Явных рисков нет</span>
          )}
        </div>
      </div>

      <dl className="admin-kv admin-kv--compact admin-littlemice-kv">
        <div>
          <dt>PID</dt>
          <dd>{asText(runtime.pid)}</dd>
        </div>
        <div>
          <dt>JVM</dt>
          <dd>{asText(runtime.vm)}</dd>
        </div>
        <div>
          <dt>Attach API</dt>
          <dd>{asText(runtime.attach?.api)}</dd>
        </div>
        <div>
          <dt>Attach self</dt>
          <dd>{asText(runtime.attach?.self)}</dd>
        </div>
        <div>
          <dt>Disable attach</dt>
          <dd>{asText(runtime.attach?.disabled)}</dd>
        </div>
        <div>
          <dt>JMX remote</dt>
          <dd>{asText(runtime.attach?.jmx)}</dd>
        </div>
        <div>
          <dt>Native modules readable</dt>
          <dd>{asText(runtime.native?.ok)}</dd>
        </div>
        <div>
          <dt>DLL count</dt>
          <dd>{asText(runtime.native?.count)}</dd>
        </div>
      </dl>

      <section className="admin-littlemice-runtime-subsection">
        <h4>Agent args</h4>
        {agentArgs.length ? (
          <div className="admin-littlemice-code-list">
            {agentArgs.map((item, index) => (
              <code key={`${asText(item)}-${index}`}>{asText(item)}</code>
            ))}
          </div>
        ) : (
          <p className="admin-inline-muted">Agent args не найдены.</p>
        )}
      </section>

      <section className="admin-littlemice-runtime-subsection">
        <h4>Нетипичные DLL</h4>
        {oddNative.length ? (
          <div className="admin-littlemice-data-table-wrap">
            <table className="admin-littlemice-data-table">
              <thead>
                <tr>
                  <th>Имя</th>
                  <th>Путь</th>
                  <th>Причина</th>
                </tr>
              </thead>
              <tbody>
                {oddNative.map((item, index) => (
                  <tr key={`${asText(item.p)}-${index}`}>
                    <td>{asText(item.n)}</td>
                    <td>
                      <code>{asText(item.p)}</code>
                    </td>
                    <td>{asText(item.r)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="admin-inline-muted">Нетипичные DLL не найдены.</p>
        )}
      </section>
    </section>
  );
}

function ClientInfoHumanView({ text }: { text: string }) {
  const info = parseClientInfo(text);
  if (!info) return <pre>{text}</pre>;

  const resourcePacks = Array.isArray(info.rp?.items) ? info.rp.items : [];
  const shaderPacks = Array.isArray(info.sp?.items) ? info.sp.items : [];
  const mods = Array.isArray(info.mods) ? info.mods : [];

  return (
    <div className="admin-littlemice-client-info">
      <dl className="admin-kv admin-kv--compact admin-littlemice-kv">
        <div>
          <dt>Время формирования</dt>
          <dd>{formatDateTime(asText(info.t))}</dd>
        </div>
        <div>
          <dt>Minecraft</dt>
          <dd>{asText(info.mc)}</dd>
        </div>
        <div>
          <dt>Тип сборки</dt>
          <dd>{asText(info.vt)}</dd>
        </div>
        <div>
          <dt>Игрок</dt>
          <dd>{asText(info.pn)}</dd>
        </div>
        <div>
          <dt>UUID профиля</dt>
          <dd>
            <code>{asText(info.pu)}</code>
          </dd>
        </div>
        <div>
          <dt>Сервер клиента</dt>
          <dd>{asText(info.srv)}</dd>
        </div>
      </dl>

      <RuntimeDiagnosticsView runtime={info.rt} />

      <section className="admin-littlemice-client-section">
        <h3>Resource packs</h3>
        {resourcePacks.length ? (
          <div className="admin-littlemice-data-table-wrap">
            <table className="admin-littlemice-data-table">
              <thead>
                <tr>
                  <th>Имя</th>
                  <th>ID</th>
                  <th>Описание</th>
                  <th>Required</th>
                  <th>Loaded</th>
                  <th>Hash</th>
                  <th>Hash no whitespace</th>
                </tr>
              </thead>
              <tbody>
                {resourcePacks.map((item, index) => (
                  <tr key={`${asText(item.id)}-${index}`}>
                    <td>{asText(item.n)}</td>
                    <td>
                      <code>{asText(item.id)}</code>
                    </td>
                    <td>{asText(item.d)}</td>
                    <td>{asText(item.r)}</td>
                    <td>{asText(item.loaded)}</td>
                    <td>
                      <code>{asText(item.h)}</code>
                    </td>
                    <td>
                      <code>{asText(item.hw)}</code>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="admin-inline-muted">Нет resource packs.</p>
        )}
      </section>

      <section className="admin-littlemice-client-section">
        <h3>Shader packs</h3>
        <p className="admin-inline-muted">
          Активный: <strong>{asText(info.sp?.active)}</strong>
        </p>
        {shaderPacks.length ? (
          <div className="admin-littlemice-data-table-wrap">
            <table className="admin-littlemice-data-table">
              <thead>
                <tr>
                  <th>Имя</th>
                  <th>Loaded</th>
                </tr>
              </thead>
              <tbody>
                {shaderPacks.map((item, index) => (
                  <tr key={`${asText(item.name)}-${index}`}>
                    <td>{asText(item.name)}</td>
                    <td>{asText(item.loaded)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="admin-inline-muted">Нет shader packs.</p>
        )}
      </section>

      <section className="admin-littlemice-client-section">
        <h3>Forge mods</h3>
        {mods.length ? (
          <div className="admin-littlemice-data-table-wrap">
            <table className="admin-littlemice-data-table">
              <thead>
                <tr>
                  <th>Mod ID</th>
                  <th>Название</th>
                  <th>Версия</th>
                  <th>Namespace</th>
                </tr>
              </thead>
              <tbody>
                {mods.map((item, index) => (
                  <tr key={`${asText(item.id)}-${index}`}>
                    <td>
                      <code>{asText(item.id)}</code>
                    </td>
                    <td>{asText(item.name)}</td>
                    <td>{asText(item.version)}</td>
                    <td>{asText(item.namespace)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="admin-inline-muted">Моды не найдены.</p>
        )}
      </section>
    </div>
  );
}

function useAuthorizedImageUrl(token: string, url: string | null) {
  const [loadedImage, setLoadedImage] = useState<{ sourceUrl: string; objectUrl: string } | null>(
    null,
  );
  const [loadError, setLoadError] = useState<{ sourceUrl: string; message: string } | null>(null);

  useEffect(() => {
    if (!url) return;

    let cancelled = false;
    let nextObjectUrl: string | null = null;
    const run = async () => {
      try {
        const blob = await getAdminLittlemiceBinaryContent(token, url);
        if (cancelled) return;
        nextObjectUrl = URL.createObjectURL(blob);
        setLoadedImage({ sourceUrl: url, objectUrl: nextObjectUrl });
        setLoadError(null);
      } catch (cause) {
        if (!cancelled)
          setLoadError({
            sourceUrl: url,
            message: toDisplayError(cause, "Не удалось загрузить изображение."),
          });
      }
    };
    void run();
    return () => {
      cancelled = true;
      if (nextObjectUrl) URL.revokeObjectURL(nextObjectUrl);
    };
  }, [token, url]);

  return {
    objectUrl: url && loadedImage?.sourceUrl === url ? loadedImage.objectUrl : null,
    error: url && loadError?.sourceUrl === url ? loadError.message : "",
  };
}

function ScreenshotPanel({
  title,
  url,
  sizeBytes,
  token,
  onOpen,
}: {
  title: string;
  url: string | null;
  sizeBytes: number | null;
  token: string;
  onOpen: (image: { title: string; src: string }) => void;
}) {
  const image = useAuthorizedImageUrl(token, url);
  const objectUrl = image.objectUrl;

  return (
    <section className="admin-littlemice-media-panel">
      <div className="admin-littlemice-media-head">
        <h3>{title}</h3>
        <small>{formatBytes(sizeBytes)}</small>
      </div>
      {!url ? <p className="admin-inline-muted">Файл не загружен.</p> : null}
      {url && !objectUrl && !image.error ? <LoadingState title="Загружаем скриншот" /> : null}
      {image.error ? <ErrorState message={image.error} /> : null}
      {objectUrl ? (
        <button
          type="button"
          className="admin-littlemice-screenshot-button"
          onClick={() => onOpen({ title, src: objectUrl })}
        >
          <img className="admin-littlemice-screenshot" src={objectUrl} alt={title} />
        </button>
      ) : null}
    </section>
  );
}

function AdminUserLittlemiceDetailView({
  token,
  userId,
  checkId,
}: {
  token: string;
  userId: string;
  checkId: string;
}) {
  const [check, setCheck] = useState<LittlemiceCheckDetailResponse | null>(null);
  const [player, setPlayer] = useState<AdminUserResponse | null | undefined>(undefined);
  const [logText, setLogText] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [logError, setLogError] = useState("");
  const [fullscreenImage, setFullscreenImage] = useState<{ title: string; src: string } | null>(
    null,
  );
  const [activeTextModal, setActiveTextModal] = useState<"clientInfo" | "log" | null>(null);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      setCheck(null);
      setPlayer(undefined);
      setLogText(null);
      setError("");
      setLogError("");
      try {
        const loaded = await getAdminLittlemiceCheck(token, checkId);
        if (cancelled) return;
        setCheck(loaded);

        void getAdminUser(token, loaded.playerUuid)
          .then((loadedPlayer) => {
            if (!cancelled) setPlayer(loadedPlayer);
          })
          .catch(() => {
            if (!cancelled) setPlayer(null);
          });

        if (loaded.logUrl) {
          try {
            const text = await getAdminLittlemiceTextContent(token, loaded.logUrl);
            if (!cancelled) setLogText(text);
          } catch (cause) {
            if (!cancelled)
              setLogError(toDisplayError(cause, "Не удалось загрузить лог проверки."));
          }
        }
      } catch (cause) {
        if (!cancelled)
          setError(toDisplayError(cause, "Не удалось загрузить проверку littlemice."));
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [checkId, token]);

  const playerMismatch = useMemo(() => check && check.playerUuid !== userId, [check, userId]);

  return (
    <div className="admin-page">
      <section className="admin-top-actions admin-page-toolbar">
        <AdminLink className="btn btn-sm" href={adminUserLittlemicePath(userId)}>
          ← К проверкам littlemice
        </AdminLink>
        <AdminLink className="btn btn-sm" href={adminUserPath(userId)}>
          К профилю игрока
        </AdminLink>
      </section>

      <section className="card admin-card">
        {!check && !error ? <LoadingState title="Загружаем проверку" /> : null}
        {error ? <ErrorState title="Проверка недоступна" message={error} /> : null}
        {check ? (
          <div className="admin-littlemice-detail">
            <div className="admin-littlemice-detail-hero">
              <h2 className="card-title">Проверка littlemice</h2>
              <p className="admin-inline-muted">
                Check ID: <code>{check.id}</code>
              </p>
              <span>
                Ник игрока: <strong>{playerLabel(player, check.playerUuid)}</strong>
              </span>
            </div>

            {playerMismatch ? (
              <p className="admin-inline-warning">
                Эта проверка относится к игроку <code>{check.playerUuid}</code>, а открыта из
                профиля <code>{userId}</code>.
              </p>
            ) : null}

            <dl className="admin-kv admin-kv--compact admin-littlemice-kv">
              <div>
                <dt>Игрок</dt>
                <dd>
                  <code>{check.playerUuid}</code>
                </dd>
              </div>
              <div>
                <dt>Сервис</dt>
                <dd>{check.serviceSystemName}</dd>
              </div>
              <div>
                <dt>Запрошена</dt>
                <dd>{formatDateTime(check.requestedAt)}</dd>
              </div>
              <div>
                <dt>Получена</dt>
                <dd>{formatDateTime(check.receivedAt)}</dd>
              </div>
              <div>
                <dt>Завершена</dt>
                <dd>{formatDateTime(check.completedAt)}</dd>
              </div>
              <div>
                <dt>Истекает</dt>
                <dd>{formatDateTime(check.expiresAt)}</dd>
              </div>
              {check.failureReason ? (
                <div>
                  <dt>Причина ошибки</dt>
                  <dd>{check.failureReason}</dd>
                </div>
              ) : null}
            </dl>

            <div className="admin-littlemice-evidence-summary">
              <div className="admin-littlemice-evidence-card">
                <span>Скриншоты</span>
                <strong>
                  {[check.screenshotUrl, check.screenshot2Url].filter(Boolean).length}/2
                </strong>
              </div>
              <div className="admin-littlemice-evidence-card">
                <span>Client info</span>
                <strong>
                  {check.clientInfoText ? formatBytes(check.clientInfoSizeBytes) : "нет"}
                </strong>
              </div>
              <div className="admin-littlemice-evidence-card">
                <span>Лог</span>
                <strong>{check.logUrl ? formatBytes(check.logSizeBytes) : "нет"}</strong>
              </div>
            </div>

            <div className="admin-littlemice-media-grid">
              <ScreenshotPanel
                title="Скриншот"
                url={check.screenshotUrl}
                sizeBytes={check.screenshotSizeBytes}
                token={token}
                onOpen={setFullscreenImage}
              />
              <ScreenshotPanel
                title="Скриншот 2"
                url={check.screenshot2Url}
                sizeBytes={check.screenshot2SizeBytes}
                token={token}
                onOpen={setFullscreenImage}
              />
            </div>

            <section className="admin-littlemice-info-grid">
              <article className="admin-littlemice-file-card">
                <div className="admin-littlemice-media-head">
                  <h3>Client info</h3>
                  <small>{formatBytes(check.clientInfoSizeBytes)}</small>
                </div>
                {check.clientInfoText ? (
                  <div className="admin-littlemice-file-actions">
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={() => setActiveTextModal("clientInfo")}
                    >
                      Открыть
                    </button>
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={() =>
                        downloadText(
                          `littlemice-${check.id}-client-info.json`,
                          check.clientInfoText ?? "",
                          "application/json;charset=utf-8",
                        )
                      }
                    >
                      Скачать
                    </button>
                  </div>
                ) : (
                  <p className="admin-inline-muted">Client info не загружен.</p>
                )}
              </article>

              <article className="admin-littlemice-file-card">
                <div className="admin-littlemice-media-head">
                  <h3>Log</h3>
                  <small>{formatBytes(check.logSizeBytes)}</small>
                </div>
                {!check.logUrl ? <p className="admin-inline-muted">Лог не загружен.</p> : null}
                {check.logUrl && logText == null && !logError ? (
                  <LoadingState title="Загружаем лог" />
                ) : null}
                {logError ? <ErrorState message={logError} /> : null}
                {logText != null ? (
                  <div className="admin-littlemice-file-actions">
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={() => setActiveTextModal("log")}
                    >
                      Открыть
                    </button>
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={() => downloadText(`littlemice-${check.id}.log`, logText)}
                    >
                      Скачать
                    </button>
                  </div>
                ) : null}
              </article>
            </section>
          </div>
        ) : null}
      </section>

      {fullscreenImage ? (
        <AppPortal>
          <div
            className="admin-littlemice-fullscreen"
            role="presentation"
            onClick={() => setFullscreenImage(null)}
          >
            <button
              type="button"
              className="admin-littlemice-fullscreen-close"
              aria-label="Закрыть"
              onClick={() => setFullscreenImage(null)}
            >
              ×
            </button>
            <img
              src={fullscreenImage.src}
              alt={fullscreenImage.title}
              onClick={(event) => event.stopPropagation()}
            />
          </div>
        </AppPortal>
      ) : null}

      {check && activeTextModal ? (
        <AppPortal>
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => setActiveTextModal(null)}
          >
            <div
              className="ui-modal admin-littlemice-text-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="littlemice-text-modal-title"
              onClick={(event) => event.stopPropagation()}
            >
              <div className="ui-modal-header">
                <h2 id="littlemice-text-modal-title" className="ui-modal-title">
                  {activeTextModal === "clientInfo" ? "Client info" : "Log"}
                </h2>
                <button
                  className="ui-modal-close"
                  type="button"
                  aria-label="Закрыть"
                  onClick={() => setActiveTextModal(null)}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body admin-littlemice-text-modal-body">
                {activeTextModal === "clientInfo" && check.clientInfoText ? (
                  <ClientInfoHumanView text={check.clientInfoText} />
                ) : null}
                {activeTextModal === "log" && logText != null ? <pre>{logText}</pre> : null}
              </div>
              <div className="ui-modal-footer">
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => {
                    if (activeTextModal === "clientInfo")
                      downloadText(
                        `littlemice-${check.id}-client-info.json`,
                        check.clientInfoText ?? "",
                        "application/json;charset=utf-8",
                      );
                    if (activeTextModal === "log")
                      downloadText(`littlemice-${check.id}.log`, logText ?? "");
                  }}
                >
                  Скачать
                </button>
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => setActiveTextModal(null)}
                >
                  Закрыть
                </button>
              </div>
            </div>
          </div>
        </AppPortal>
      ) : null}
    </div>
  );
}

export default function AdminUserLittlemiceChecksView({
  token,
  userId,
  checkId,
}: {
  token: string;
  userId: string;
  checkId?: string;
}) {
  if (checkId) {
    return <AdminUserLittlemiceDetailView token={token} userId={userId} checkId={checkId} />;
  }

  return <AdminUserLittlemiceListView token={token} userId={userId} />;
}
