const ALLOWED_HOSTS = new Set([
  "youtube.com",
  "www.youtube.com",
  "m.youtube.com",
  "youtu.be",
  "www.youtu.be",
]);

const VIDEO_ID_RE = /^[A-Za-z0-9_-]{11}$/;

function extractVideoId(url: URL): string | null {
  const host = url.hostname.toLowerCase();

  if (host === "youtu.be" || host === "www.youtu.be") {
    const id = url.pathname.split("/").filter(Boolean)[0] ?? "";
    return VIDEO_ID_RE.test(id) ? id : null;
  }

  if (host === "youtube.com" || host === "www.youtube.com" || host === "m.youtube.com") {
    const v = url.searchParams.get("v");
    if (v && VIDEO_ID_RE.test(v)) {
      return v;
    }

    const parts = url.pathname.split("/").filter(Boolean);
    if (parts[0] === "shorts" && parts[1] && VIDEO_ID_RE.test(parts[1])) {
      return parts[1];
    }
  }

  return null;
}

function parseYouTubeUrl(url: string): string | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }

  if (!ALLOWED_HOSTS.has(parsed.hostname.toLowerCase())) {
    return null;
  }

  return extractVideoId(parsed);
}

/** Returns true if `url` is a YouTube watch, short, or youtu.be link with a valid video id. */
export function isYouTubeUrl(url: string): boolean {
  return parseYouTubeUrl(url) !== null;
}

/** Canonical `https://www.youtube.com/watch?v=ID`, or null if not a valid YouTube URL. */
export function normalizeYouTubeUrl(url: string): string | null {
  const id = parseYouTubeUrl(url);
  if (!id) return null;
  return `https://www.youtube.com/watch?v=${id}`;
}
