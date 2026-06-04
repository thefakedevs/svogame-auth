import { useEffect, useRef, useState, type ChangeEvent } from "react";
import toast from "react-hot-toast";
import { createAdminShopProduct } from "../../api/admin";
import { toDisplayError } from "../../api/http";
import {
  createAdminAsset,
  patchAdminAsset,
  uploadAdminAssetImage,
  type SkinRarity,
} from "../../api/inventory";
import { adminShopProductPath } from "../../routes/paths";
import AdminLink from "./AdminLink";

function normalizeAssetKey(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9_-]/g, "");
}

const defaultLocale = "ru-RU";

export default function AdminGunskinShopWizard({ token }: { token: string }) {
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const [assetKey, setAssetKey] = useState("");
  const [shopProductKey, setShopProductKey] = useState("");
  const [weaponKey, setWeaponKey] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [description, setDescription] = useState("");
  const [rarity, setRarity] = useState<SkinRarity>("common");
  const [priceRub, setPriceRub] = useState("");
  const [imageFile, setImageFile] = useState<File | null>(null);
  const [imagePreviewUrl, setImagePreviewUrl] = useState<string | null>(null);

  const [assetActive, setAssetActive] = useState(true);
  const [assetPublic, setAssetPublic] = useState(true);
  const [shopActive, setShopActive] = useState(true);
  const [shopPublic, setShopPublic] = useState(false);

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [lastProductId, setLastProductId] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      if (imagePreviewUrl) URL.revokeObjectURL(imagePreviewUrl);
    };
  }, [imagePreviewUrl]);

  const onPickFile = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null;
    event.target.value = "";
    setImagePreviewUrl((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return null;
    });
    setImageFile(null);
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      toast.error("Нужен файл изображения (PNG, JPG или WebP).");
      return;
    }
    if (file.size > 2 * 1024 * 1024) {
      toast.error("Изображение не больше 2 МБ.");
      return;
    }
    setImageFile(file);
    setImagePreviewUrl(URL.createObjectURL(file));
  };

  const resetForm = () => {
    setAssetKey("");
    setShopProductKey("");
    setWeaponKey("");
    setDisplayName("");
    setDescription("");
    setRarity("common");
    setPriceRub("");
    setImageFile(null);
    setImagePreviewUrl((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return null;
    });
    setAssetActive(true);
    setAssetPublic(true);
    setShopActive(true);
    setShopPublic(false);
  };

  const submit = async () => {
    const key = assetKey.trim();
    const productKeyRaw = shopProductKey.trim();
    const productKey = normalizeAssetKey(productKeyRaw || key);
    const wk = weaponKey.trim();
    const name = displayName.trim();
    const price = Number(priceRub);

    if (!key) {
      toast.error("Укажите key ассета (латиница, цифры, _ и -).");
      return;
    }
    if (key !== normalizeAssetKey(key)) {
      toast.error("Key ассета: только строчные латинские буквы, цифры, _ и -.");
      return;
    }
    if (!productKey) {
      toast.error("Укажите ключ товара в магазине или заполните key ассета.");
      return;
    }
    if (!wk) {
      toast.error("Укажите weaponKey.");
      return;
    }
    if (!name) {
      toast.error("Укажите название.");
      return;
    }
    if (!Number.isFinite(price) || price <= 0) {
      toast.error("Цена должна быть целым числом рублей больше нуля.");
      return;
    }
    if (!imageFile) {
      toast.error("Выберите изображение скина.");
      return;
    }

    setIsSubmitting(true);
    setLastProductId(null);

    let createdAssetId: string | null = null;

    try {
      const created = await createAdminAsset(token, {
        key,
        display_name: name,
        description: description.trim() || null,
        asset_kind: "skin",
        ownership_model: "entitlement",
        is_currency: false,
        is_user_purchasable: true,
        is_public: assetPublic,
        rarity,
        weaponKey: wk,
        metadata: {},
      });
      createdAssetId = created.id;

      if (!assetActive) {
        await patchAdminAsset(token, created.id, { is_active: false });
      }

      await uploadAdminAssetImage(token, created.id, imageFile);

      const product = await createAdminShopProduct(token, {
        key: productKey,
        asset_key: key,
        price_rub: Math.trunc(price),
        locales: [
          {
            locale: defaultLocale,
            name,
            description: description.trim() || null,
          },
        ],
        is_active: shopActive,
        is_public: shopPublic,
      });

      setLastProductId(product.id);
      toast.success("Скин и товар магазина созданы.");
      resetForm();
    } catch (cause) {
      const hint =
        createdAssetId != null
          ? ` Ассет уже создан (id ${createdAssetId}); при необходимости дозагрузите картинку или создайте товар вручную во вкладке «Магазин».`
          : "";
      toast.error(toDisplayError(cause, "Не удалось выполнить шаг.") + hint);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <section className="card admin-card admin-skin-wizard" aria-label="Новый скин и товар">
      <h2 className="card-title">Новый скин в каталог и магазин</h2>
      <p className="admin-inline-muted">
        Создаётся ассет вида <code>skin</code> с моделью владения <code>entitlement</code>,
        загружается превью, затем привязывается товар магазина. По умолчанию товар{" "}
        <strong>не публичный</strong> в витрине магазина, ассет — публичный в каталоге (если не
        снять флажок).
      </p>

      <h3 className="card-title admin-skin-wizard__subtitle">Данные скина</h3>
      <div className="admin-asset-form-grid">
        <input
          className="ui-input"
          value={assetKey}
          onChange={(e) => setAssetKey(normalizeAssetKey(e.target.value))}
          placeholder="key ассета"
          autoComplete="off"
        />
        <input
          className="ui-input"
          value={shopProductKey}
          onChange={(e) => setShopProductKey(normalizeAssetKey(e.target.value))}
          placeholder="ключ товара (пусто = как у ассета)"
          autoComplete="off"
        />
        <input
          className="ui-input"
          value={weaponKey}
          onChange={(e) => setWeaponKey(e.target.value)}
          placeholder="weaponKey"
        />
        <select
          className="ui-input"
          value={rarity}
          onChange={(e) => setRarity(e.target.value as SkinRarity)}
        >
          <option value="common">common</option>
          <option value="rare">rare</option>
          <option value="legendary">legendary</option>
        </select>
        <input
          className="ui-input"
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder="Название"
        />
        <input
          className="ui-input"
          value={priceRub}
          onChange={(e) => setPriceRub(e.target.value.replace(/\D/g, ""))}
          placeholder="Цена, ₽"
          inputMode="numeric"
        />
      </div>
      <textarea
        className="ui-input admin-asset-metadata"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="Описание (ru-RU для магазина)"
        rows={3}
      />

      <h3 className="card-title admin-skin-wizard__subtitle">Превью</h3>
      <div className="admin-skin-wizard__image-row">
        <input
          ref={fileInputRef}
          className="admin-hidden-file-input"
          type="file"
          accept="image/png,image/jpeg,image/webp,image/*"
          onChange={onPickFile}
        />
        <button type="button" className="btn btn-sm" onClick={() => fileInputRef.current?.click()}>
          Выбрать файл
        </button>
        {imagePreviewUrl ? (
          <span className="admin-skin-wizard__thumb-wrap">
            <img src={imagePreviewUrl} alt="" className="admin-skin-wizard__thumb" />
          </span>
        ) : (
          <span className="admin-inline-muted">PNG, JPG или WebP до 2 МБ.</span>
        )}
      </div>

      <h3 className="card-title admin-skin-wizard__subtitle">Ассет в каталоге</h3>
      <div className="admin-asset-flags">
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={assetActive}
            onChange={(e) => setAssetActive(e.target.checked)}
          />
          Ассет активен
        </label>
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={assetPublic}
            onChange={(e) => setAssetPublic(e.target.checked)}
          />
          Ассет публичный (каталог / API)
        </label>
      </div>

      <h3 className="card-title admin-skin-wizard__subtitle">Магазин</h3>
      <div className="admin-asset-flags">
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={shopActive}
            onChange={(e) => setShopActive(e.target.checked)}
          />
          Товар активен
        </label>
        <label className="admin-checkbox-row">
          <input
            type="checkbox"
            checked={shopPublic}
            onChange={(e) => setShopPublic(e.target.checked)}
          />
          Товар публичный в магазине (витрина)
        </label>
      </div>

      <div className="admin-skin-wizard__actions">
        <button
          type="button"
          className="btn primary"
          disabled={isSubmitting}
          onClick={() => void submit()}
        >
          {isSubmitting ? "Создаём…" : "Создать скин и товар"}
        </button>
        {lastProductId ? (
          <AdminLink className="btn btn-sm" href={adminShopProductPath(lastProductId)}>
            Открыть последний товар
          </AdminLink>
        ) : null}
      </div>
    </section>
  );
}
