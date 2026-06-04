import type { PropsWithChildren } from "react";
import { createPortal } from "react-dom";

function getPortalRoot() {
  if (typeof document === "undefined") {
    return null;
  }

  return (
    document.querySelector(".ui-kit-page.app-shell") ??
    document.querySelector(".ui-kit-page") ??
    document.getElementById("root") ??
    document.body
  );
}

export default function AppPortal({ children }: PropsWithChildren) {
  const portalRoot = getPortalRoot();
  if (!portalRoot) {
    return null;
  }

  return createPortal(children, portalRoot);
}
