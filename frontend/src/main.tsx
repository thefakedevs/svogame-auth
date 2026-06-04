import { createRoot } from "react-dom/client";
import AccountApp from "./app/AccountApp";
import { initAnalytics, installAnalyticsInteractionTracking } from "./services/analytics";
import { bootstrapAuthSession } from "./shared/session/auth-session";
import "./styles/global.css";

const container = document.getElementById("root");

if (!container) {
  throw new Error("Root container #root was not found.");
}

bootstrapAuthSession();
initAnalytics();
installAnalyticsInteractionTracking();

createRoot(container).render(<AccountApp />);
