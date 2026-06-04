/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_TARGET?: string;
  readonly VITE_UMAMI_ENABLED?: string;
  readonly VITE_UMAMI_SCRIPT_URL?: string;
  readonly VITE_UMAMI_WEBSITE_ID?: string;
  readonly VITE_UMAMI_HOST_URL?: string;
  readonly VITE_UMAMI_DOMAINS?: string;
  readonly VITE_UMAMI_TAG?: string;
  readonly VITE_UMAMI_DO_NOT_TRACK?: string;
}
