import type { AuthorizedAuthResponse } from "../../../api/auth";
import { paths } from "../../../routes/paths";
import { navigateTo } from "../../../shared/navigation/history";
import { storeAuthorizedSession } from "../../../shared/session/auth-session";

export async function finishAuthDelivery(auth: AuthorizedAuthResponse) {
  storeAuthorizedSession({
    token: auth.accessToken,
    user: auth.user,
  });

  if (auth.deliveryMethod === "polling") {
    navigateTo(paths.profile, { replace: true });
    return;
  }

  try {
    const parsed = new URL(auth.deliveryTarget, window.location.origin);
    if (parsed.origin === window.location.origin) {
      navigateTo(parsed.pathname + parsed.search, { replace: true });
      return;
    }

    parsed.searchParams.set("token", auth.accessToken);
    window.location.href = parsed.toString();
  } catch {
    const tokenParam = auth.deliveryTarget.includes("?") ? "&" : "?";
    window.location.href = `${auth.deliveryTarget}${tokenParam}token=${encodeURIComponent(auth.accessToken)}`;
  }
}
