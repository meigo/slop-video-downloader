import { formatFilenameTime } from "./time";

/** Strip path-unsafe chars, collapse whitespace. Empty result becomes "clip". */
export function sanitizeTitle(title: string): string {
  // Replace / \ ? % * : | " < > and control chars with space
  const cleaned = title.replace(/[/\\?%*:|"<>\x00-\x1f\x7f]/g, " ");
  const collapsed = cleaned.replace(/\s+/g, " ").trim();
  return collapsed.length > 0 ? collapsed : "clip";
}

/** `{sanitized}_{start}-{end}.mp4` using filename time tokens. */
export function buildClipFilename(
  title: string,
  startSecs: number,
  endSecs: number,
): string {
  const safe = sanitizeTitle(title);
  const start = formatFilenameTime(startSecs);
  const end = formatFilenameTime(endSecs);
  return `${safe}_${start}-${end}.mp4`;
}
