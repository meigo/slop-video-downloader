/** Supported source families (public videos only). */
export type SourceFamily = "youtube" | "vimeo";

const YOUTUBE_HOSTS = new Set([
  "youtube.com",
  "www.youtube.com",
  "m.youtube.com",
  "youtu.be",
  "www.youtu.be",
]);

const VIMEO_HOSTS = new Set(["vimeo.com", "www.vimeo.com", "player.vimeo.com"]);

const YT_VIDEO_ID_RE = /^[A-Za-z0-9_-]{11}$/;
const VIMEO_ID_RE = /^\d+$/;

export const UNSUPPORTED_SITE_MESSAGE =
  "Unsupported site in this version. Supported: YouTube, Vimeo (public videos).";

function parseHttpUrl(url: string): URL | null {
  try {
    const parsed = new URL(url.trim());
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

function extractYouTubeId(url: URL): string | null {
  const host = url.hostname.toLowerCase();

  if (host === "youtu.be" || host === "www.youtu.be") {
    const id = url.pathname.split("/").filter(Boolean)[0] ?? "";
    return YT_VIDEO_ID_RE.test(id) ? id : null;
  }

  if (host === "youtube.com" || host === "www.youtube.com" || host === "m.youtube.com") {
    const v = url.searchParams.get("v");
    if (v && YT_VIDEO_ID_RE.test(v)) {
      return v;
    }

    const parts = url.pathname.split("/").filter(Boolean);
    if (parts[0] === "shorts" && parts[1] && YT_VIDEO_ID_RE.test(parts[1])) {
      return parts[1];
    }
  }

  return null;
}

/** Numeric Vimeo id when path is /123456789 or /video/123456789. */
function extractVimeoId(url: URL): string | null {
  const parts = url.pathname.split("/").filter(Boolean);
  if (parts.length === 0) return null;
  if (parts[0] === "video" && parts[1] && VIMEO_ID_RE.test(parts[1])) {
    return parts[1];
  }
  if (VIMEO_ID_RE.test(parts[0])) {
    return parts[0];
  }
  return null;
}

export function sourceFamily(url: string): SourceFamily | null {
  const parsed = parseHttpUrl(url);
  if (!parsed) return null;
  const host = parsed.hostname.toLowerCase();
  if (YOUTUBE_HOSTS.has(host)) return "youtube";
  if (VIMEO_HOSTS.has(host)) return "vimeo";
  return null;
}

/** True if URL is on an allowlisted host and looks like a single video page. */
export function isSupportedUrl(url: string): boolean {
  return normalizeSourceUrl(url) !== null;
}

/**
 * Normalize to a stable https URL for yt-dlp, or null if unsupported.
 * YouTube → https://www.youtube.com/watch?v=ID
 * Vimeo → https://vimeo.com/{id} when parseable
 */
export function normalizeSourceUrl(url: string): string | null {
  const parsed = parseHttpUrl(url);
  if (!parsed) return null;

  const host = parsed.hostname.toLowerCase();
  const family = sourceFamily(url);
  if (!family) return null;

  if (family === "youtube") {
    if (!YOUTUBE_HOSTS.has(host)) return null;
    const id = extractYouTubeId(parsed);
    if (!id) return null;
    return `https://www.youtube.com/watch?v=${id}`;
  }

  // vimeo
  if (!VIMEO_HOSTS.has(host)) return null;
  const id = extractVimeoId(parsed);
  if (id) {
    return `https://vimeo.com/${id}`;
  }
  // Allow other public Vimeo paths (channels/showcase) only if host is vimeo — require id for safety
  return null;
}

// --- Back-compat aliases used by older call sites / tests ---

/** @deprecated Prefer isSupportedUrl */
export function isYouTubeUrl(url: string): boolean {
  return sourceFamily(url) === "youtube" && normalizeSourceUrl(url) !== null;
}

/** @deprecated Prefer normalizeSourceUrl */
export function normalizeYouTubeUrl(url: string): string | null {
  if (sourceFamily(url) !== "youtube") return null;
  return normalizeSourceUrl(url);
}
