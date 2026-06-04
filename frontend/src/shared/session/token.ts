export interface PowData {
  solution: string;
  prefix: string;
}

export function extractTokenFromUrl(searchParams: URLSearchParams): string | null {
  const token = searchParams.get("token");
  return token && token.trim() !== "" ? token : null;
}

export function validateTokenFormat(token: string): boolean {
  if (!token || typeof token !== "string") {
    return false;
  }

  const parts = token.split(".");
  if (parts.length !== 3) {
    return false;
  }

  const base64UrlPattern = /^[A-Za-z0-9_-]+$/;
  return parts.every((part) => part.length > 0 && base64UrlPattern.test(part));
}
