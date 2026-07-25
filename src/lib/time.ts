function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

/** Display timestamp: `MM:SS` or `H:MM:SS`, with optional tenths when fractional. */
export function formatTimestamp(secs: number): string {
  const total = Math.max(0, secs);
  const whole = Math.floor(total);
  const tenths = Math.floor((total - whole) * 10 + 1e-9);

  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const seconds = whole % 60;

  let base: string;
  if (hours > 0) {
    base = `${hours}:${pad2(minutes)}:${pad2(seconds)}`;
  } else {
    base = `${pad2(minutes)}:${pad2(seconds)}`;
  }

  if (tenths > 0) {
    return `${base}.${tenths}`;
  }
  return base;
}

/** Filename-safe time token, e.g. `01m12s` or `01h01m01s` (whole seconds). */
export function formatFilenameTime(secs: number): string {
  const whole = Math.max(0, Math.floor(secs));
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const seconds = whole % 60;

  if (hours > 0) {
    return `${pad2(hours)}h${pad2(minutes)}m${pad2(seconds)}s`;
  }
  return `${pad2(minutes)}m${pad2(seconds)}s`;
}

/**
 * Ensures start < end within [0, duration].
 * Swaps if start > end; if equal, bumps end (or shrinks start if at duration).
 */
export function clampRange(
  start: number,
  end: number,
  duration: number,
): { start: number; end: number } {
  let s = start;
  let e = end;

  if (s > e) {
    [s, e] = [e, s];
  }

  const dur = Math.max(0, duration);
  s = Math.min(Math.max(0, s), dur);
  e = Math.min(Math.max(0, e), dur);

  if (s === e) {
    const bump = Math.min(0.1, dur - s);
    if (bump > 0) {
      e = s + bump;
    } else if (dur > 0) {
      // At the end: shrink start slightly so end > start.
      s = Math.max(0, e - 0.1);
    }
  }

  return { start: s, end: e };
}
