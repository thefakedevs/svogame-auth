import { useSyncExternalStore } from "react";

const NAVIGATION_EVENT = "svocraft:navigation";

function dispatchNavigationEvent() {
  window.dispatchEvent(new Event(NAVIGATION_EVENT));
}

function subscribe(onStoreChange: () => void) {
  if (typeof window === "undefined") {
    return () => {};
  }

  window.addEventListener("popstate", onStoreChange);
  window.addEventListener(NAVIGATION_EVENT, onStoreChange);

  return () => {
    window.removeEventListener("popstate", onStoreChange);
    window.removeEventListener(NAVIGATION_EVENT, onStoreChange);
  };
}

function getPathnameSnapshot() {
  return typeof window === "undefined" ? "" : window.location.pathname;
}

function getSearchSnapshot() {
  return typeof window === "undefined" ? "" : window.location.search;
}

function getServerSnapshot() {
  return "";
}

export function usePathname() {
  return useSyncExternalStore(subscribe, getPathnameSnapshot, getServerSnapshot);
}

export function useSearch() {
  return useSyncExternalStore(subscribe, getSearchSnapshot, getServerSnapshot);
}

export function replaceUrl(url: string) {
  if (typeof window === "undefined") return;
  window.history.replaceState(null, "", url);
  dispatchNavigationEvent();
}

export function pushUrl(url: string) {
  if (typeof window === "undefined") return;
  window.history.pushState(null, "", url);
  dispatchNavigationEvent();
}

export function navigateTo(url: string, options?: { replace?: boolean }) {
  if (options?.replace) {
    window.location.replace(url);
    return;
  }

  window.location.assign(url);
}

export function currentAppPath() {
  if (typeof window === "undefined") {
    return "/";
  }

  return `${window.location.pathname}${window.location.search}`;
}
