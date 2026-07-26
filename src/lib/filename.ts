import { formatFilenameTime } from "./time";

/** Strip path-unsafe chars, collapse whitespace. Empty result becomes "clip". */
export function sanitizeTitle(title: string): string {
  // Replace / \ ? % * : | " < > and control chars with space
  const cleaned = title.replace(/[/\\?%*:|"<>\x00-\x1f\x7f]/g, " ");
  const collapsed = cleaned.replace(/\s+/g, " ").trim();
  return collapsed.length > 0 ? collapsed : "clip";
}

/** `{sanitized}_{start}-{end}.{ext}` using filename time tokens. Default ext `mp4`. */
export function buildClipFilename(
  title: string,
  startSecs: number,
  endSecs: number,
  ext: string = "mp4",
): string {
  const safe = sanitizeTitle(title);
  const start = formatFilenameTime(startSecs);
  const end = formatFilenameTime(endSecs);
  const cleanExt = (ext.startsWith(".") ? ext.slice(1) : ext) || "mp4";
  return `${safe}_${start}-${end}.${cleanExt}`;
}
