import { useEffect, useState } from "react";
import {
  getAdminUserLootboxOpenHistory,
  type LootboxOpenHistoryResponse,
} from "../../api/lootboxes";
import { toDisplayError } from "../../api/http";
import { adminUserPath } from "../../routes/paths";
import ErrorState from "../ErrorState";
import LoadingState from "../LoadingState";
import AdminLink from "./AdminLink";

const dateTimeFormatter = new Intl.DateTimeFormat("ru-RU", {
  day: "2-digit",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

function formatDateTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : dateTimeFormatter.format(date).replace(", ", " ");
}

function formatReward(item: LootboxOpenHistoryResponse) {
  const reward = item.reward;
  if (reward.amount != null) return `${reward.displayName} x${reward.amount}`;
  if (reward.durationSeconds != null)
    return `${reward.displayName} на ${reward.durationSeconds} сек.`;
  return reward.displayName;
}

export default function AdminUserLootboxHistoryView({
  token,
  userId,
}: {
  token: string;
  userId: string;
}) {
  const [items, setItems] = useState<LootboxOpenHistoryResponse[] | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      setItems(null);
      setError("");
      try {
        const history = await getAdminUserLootboxOpenHistory(token, userId);
        if (!cancelled) setItems(history);
      } catch (cause) {
        if (!cancelled)
          setError(toDisplayError(cause, "Не удалось загрузить историю открытий кейсов."));
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [token, userId]);

  return (
    <div className="admin-page">
      <section className="admin-top-actions">
        <AdminLink className="btn btn-sm" href={adminUserPath(userId)}>
          ← К профилю игрока
        </AdminLink>
      </section>

      <section className="card admin-card">
        <h2 className="card-title">Открытые кейсы игрока</h2>
        <p className="card-text">
          User ID: <code>{userId}</code>
        </p>

        {!items && !error ? <LoadingState title="Загружаем историю открытий" /> : null}
        {error ? <ErrorState title="История недоступна" message={error} /> : null}
        {items ? (
          items.length > 0 ? (
            <div className="admin-audit-table-wrap">
              <table className="admin-audit-table">
                <thead>
                  <tr>
                    <th>ID</th>
                    <th>Кейс</th>
                    <th>Награда</th>
                    <th>Тип</th>
                    <th>Actor</th>
                    <th>Рулетка</th>
                    <th>Открыт</th>
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => (
                    <tr key={item.id}>
                      <td>{item.id}</td>
                      <td>
                        <code>{item.lootboxAssetKey}</code>
                      </td>
                      <td>
                        <strong>{formatReward(item)}</strong>
                        <br />
                        <small>
                          <code>{item.reward.assetKey}</code>
                          {item.reward.title ? ` · ${item.reward.title}` : ""}
                        </small>
                      </td>
                      <td>{item.reward.ownershipModel}</td>
                      <td>
                        {item.actorKind}
                        {item.actorServiceName ? ` · ${item.actorServiceName}` : ""}
                      </td>
                      <td>
                        {item.winnerIndex + 1}/{item.feedLength}
                      </td>
                      <td>{formatDateTime(item.openedAt)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="admin-inline-muted">Игрок еще не открывал кейсы.</p>
          )
        ) : null}
      </section>
    </div>
  );
}
