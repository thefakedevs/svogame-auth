import type { ProfileDashboardData } from "../../../components/profile/types";
import { formatImageLimit } from "./format";

export function sanitizeSquadName(value: string) {
  return value.replace(/[^A-Za-z\u0400-\u04FF -]/g, "");
}

export function hasInvalidSquadNameBoundary(value: string) {
  return value.startsWith("-") || value.endsWith("-");
}

export function isInvalidSquadNameLength(value: string) {
  return value.length < 4 || value.length > 16;
}

function normalizeSquadNamePattern(pattern: string) {
  return pattern
    .replace(/\\p\{Latin\}/g, "\\p{Script=Latin}")
    .replace(/\\p\{Cyrillic\}/g, "\\p{Script=Cyrillic}");
}

export function getSquadNameRegex(pattern: string) {
  try {
    return new RegExp(normalizeSquadNamePattern(pattern), "u");
  } catch {
    return null;
  }
}

export function hasInvalidSquadNameByConfig(
  value: string,
  config: ProfileDashboardData["squadConfig"],
) {
  if (!value) return false;

  const nameRegex = getSquadNameRegex(config.nameRegex);

  return (
    value.length < config.nameMinChars ||
    value.length > config.nameMaxChars ||
    (nameRegex ? !nameRegex.test(value) : false)
  );
}

export async function validateSquadImageByConfig(
  file: File,
  config: ProfileDashboardData["squadConfig"],
) {
  if (file.size > config.imageMaxBytes) {
    return `Файл должен быть не больше ${formatImageLimit(config.imageMaxBytes)}.`;
  }

  const objectUrl = URL.createObjectURL(file);

  try {
    const { width, height } = await new Promise<{ width: number; height: number }>(
      (resolve, reject) => {
        const image = new Image();
        image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight });
        image.onerror = () => reject(new Error("Не удалось прочитать изображение."));
        image.src = objectUrl;
      },
    );

    if (width > config.imageMaxWidth || height > config.imageMaxHeight) {
      return `Изображение должно быть не больше ${config.imageMaxWidth}x${config.imageMaxHeight}px.`;
    }

    return null;
  } finally {
    URL.revokeObjectURL(objectUrl);
  }
}
