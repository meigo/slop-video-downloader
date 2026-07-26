/**
 * YouTube helpers — thin re-exports of `source.ts` for back-compat.
 * Prefer importing from `$lib/source` for multi-source support.
 */
export {
  isYouTubeUrl,
  normalizeYouTubeUrl,
  isSupportedUrl,
  normalizeSourceUrl,
  sourceFamily,
  UNSUPPORTED_SITE_MESSAGE,
} from "./source";
export type { SourceFamily } from "./source";
