import { useEffect, useRef, useState } from "react";
import "../ui/ui.css";
import "./UiKitPage.css";
import UiKitStaticSections from "./ui-kit/UiKitStaticSections";

export default function UiKitPage() {
  const [dismissibleVisible, setDismissibleVisible] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [paginationPage, setPaginationPage] = useState(3);
  const paginationTotal = 7;
  const [uploadDrag, setUploadDrag] = useState(false);
  const [uploadFileHint, setUploadFileHint] = useState<string | null>(null);
  const uploadDragDepth = useRef(0);

  useEffect(() => {
    if (!modalOpen) return;

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setModalOpen(false);
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [modalOpen]);

  const preventUploadDefaults = (event: React.DragEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const onUploadDragEnter = (event: React.DragEvent) => {
    preventUploadDefaults(event);
    uploadDragDepth.current += 1;
    setUploadDrag(true);
  };

  const onUploadDragLeave = (event: React.DragEvent) => {
    preventUploadDefaults(event);
    uploadDragDepth.current -= 1;
    if (uploadDragDepth.current <= 0) {
      uploadDragDepth.current = 0;
      setUploadDrag(false);
    }
  };

  const onUploadDrop = (event: React.DragEvent) => {
    preventUploadDefaults(event);
    uploadDragDepth.current = 0;
    setUploadDrag(false);
    const { files } = event.dataTransfer;
    if (files?.length) {
      setUploadFileHint(
        Array.from(files)
          .map((file) => file.name)
          .join(", "),
      );
    }
  };

  const onUploadInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const { files } = event.target;
    if (files?.length) {
      setUploadFileHint(
        Array.from(files)
          .map((file) => file.name)
          .join(", "),
      );
      return;
    }

    setUploadFileHint(null);
  };

  return (
    <>
      <div className="ui-kit-vhs" aria-hidden />
      <div className="page ui-kit-page">
        <header className="ui-kit-hero">
          <h1>UI kit</h1>
          <p className="ui-muted">
            Острые углы, оранжево-синий акцент, карточки #19191C, размытие 3px и VHS-фон.
          </p>
        </header>

        <UiKitStaticSections
          dismissibleVisible={dismissibleVisible}
          onShowDismissible={() => setDismissibleVisible(true)}
          onHideDismissible={() => setDismissibleVisible(false)}
        />

        <section className="ui-section" aria-labelledby="section-upload">
          <h2 id="section-upload" className="ui-section-title">
            Upload
          </h2>
          <div
            className={`ui-upload ${uploadDrag ? "ui-upload--drag" : ""}`}
            onDragEnter={onUploadDragEnter}
            onDragLeave={onUploadDragLeave}
            onDragOver={preventUploadDefaults}
            onDrop={onUploadDrop}
          >
            <input
              id="ui-kit-upload"
              className="ui-upload-input"
              type="file"
              multiple
              accept="image/png,image/jpeg,image/gif,image/webp"
              onChange={onUploadInputChange}
            />
            <label htmlFor="ui-kit-upload" className="ui-upload-label">
              <span className="ui-upload-icon" aria-hidden>
                ⬆
              </span>
              <span className="ui-upload-title">Перетащите файлы или выберите на диске</span>
              <span className="ui-upload-hint">PNG, JPG, GIF, WebP · несколько файлов</span>
              {uploadFileHint ? (
                <span className="ui-upload-files">Выбрано: {uploadFileHint}</span>
              ) : null}
            </label>
          </div>
        </section>

        <section className="ui-section" aria-labelledby="section-pagination">
          <h2 id="section-pagination" className="ui-section-title">
            Pagination
          </h2>
          <nav aria-label="Нумерация страниц">
            <ul className="ui-pagination">
              <li>
                <button
                  type="button"
                  className="ui-pagination-btn"
                  aria-label="Предыдущая страница"
                  disabled={paginationPage <= 1}
                  onClick={() => setPaginationPage((page) => Math.max(1, page - 1))}
                >
                  ‹
                </button>
              </li>
              {Array.from({ length: paginationTotal }, (_, index) => index + 1).map((page) => (
                <li key={page}>
                  <button
                    type="button"
                    className="ui-pagination-btn"
                    aria-label={`Страница ${page}`}
                    aria-current={paginationPage === page ? "page" : undefined}
                    onClick={() => setPaginationPage(page)}
                  >
                    {page}
                  </button>
                </li>
              ))}
              <li>
                <button
                  type="button"
                  className="ui-pagination-btn"
                  aria-label="Следующая страница"
                  disabled={paginationPage >= paginationTotal}
                  onClick={() => setPaginationPage((page) => Math.min(paginationTotal, page + 1))}
                >
                  ›
                </button>
              </li>
            </ul>
          </nav>
          <p className="ui-muted" style={{ fontSize: "0.8rem", marginTop: "0.75rem" }}>
            Текущая страница: {paginationPage} / {paginationTotal}
          </p>
        </section>

        <section className="ui-section" aria-labelledby="section-modal">
          <h2 id="section-modal" className="ui-section-title">
            Modal
          </h2>
          <button type="button" className="btn primary" onClick={() => setModalOpen(true)}>
            Открыть окно
          </button>
        </section>

        {modalOpen ? (
          <div
            className="ui-modal-backdrop"
            role="presentation"
            onClick={() => setModalOpen(false)}
          >
            <div
              className="ui-modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="ui-kit-modal-title"
              onClick={(event) => event.stopPropagation()}
            >
              <div className="ui-modal-header">
                <h2 id="ui-kit-modal-title" className="ui-modal-title">
                  Заголовок окна
                </h2>
                <button
                  type="button"
                  className="ui-modal-close"
                  aria-label="Закрыть"
                  onClick={() => setModalOpen(false)}
                >
                  ×
                </button>
              </div>
              <div className="ui-modal-body">
                Содержимое модального окна. Клик по затемнению или Escape закрывает окно. Для
                продуктового режима стоит добавить ловушку фокуса.
              </div>
              <div className="ui-modal-footer">
                <button type="button" className="btn btn-sm" onClick={() => setModalOpen(false)}>
                  Отмена
                </button>
                <button
                  type="button"
                  className="btn primary btn-sm"
                  onClick={() => setModalOpen(false)}
                >
                  Ок
                </button>
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
