import { describe, expect, it } from "vitest";
import {
  isSupportedUrl,
  isYouTubeUrl,
  normalizeSourceUrl,
  normalizeYouTubeUrl,
  sourceFamily,
} from "./source";

describe("isSupportedUrl / sourceFamily", () => {
  it("accepts YouTube watch, youtu.be, shorts", () => {
    expect(isSupportedUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toBe(true);
    expect(isSupportedUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(true);
    expect(isSupportedUrl("https://www.youtube.com/shorts/dQw4w9WgXcQ")).toBe(true);
    expect(sourceFamily("https://youtu.be/dQw4w9WgXcQ")).toBe("youtube");
  });

  it("accepts Vimeo numeric ids", () => {
    expect(isSupportedUrl("https://vimeo.com/123456789")).toBe(true);
    expect(isSupportedUrl("https://www.vimeo.com/123456789")).toBe(true);
    expect(isSupportedUrl("https://player.vimeo.com/video/123456789")).toBe(true);
    expect(sourceFamily("https://vimeo.com/123456789")).toBe("vimeo");
  });

  it("accepts X / Twitter status URLs", () => {
    expect(isSupportedUrl("https://x.com/user/status/1234567890123456789")).toBe(true);
    expect(isSupportedUrl("https://twitter.com/user/status/1234567890123456789")).toBe(true);
    expect(isSupportedUrl("https://x.com/i/status/1234567890123456789")).toBe(true);
    expect(isSupportedUrl("https://mobile.twitter.com/i/web/status/1234567890123456789")).toBe(
      true,
    );
    expect(sourceFamily("https://x.com/user/status/1234567890123456789")).toBe("x");
  });

  it("rejects unsupported and invalid", () => {
    expect(isSupportedUrl("https://instagram.com/p/abc")).toBe(false);
    expect(isSupportedUrl("https://vimeo.com/")).toBe(false);
    expect(isSupportedUrl("https://x.com/home")).toBe(false);
    expect(isSupportedUrl("https://example.com")).toBe(false);
    expect(isSupportedUrl("not a url")).toBe(false);
  });
});

describe("normalizeSourceUrl", () => {
  it("normalizes youtu.be to watch URL", () => {
    expect(normalizeSourceUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    );
  });

  it("normalizes Vimeo player embed to vimeo.com/id", () => {
    expect(normalizeSourceUrl("https://player.vimeo.com/video/123456789")).toBe(
      "https://vimeo.com/123456789",
    );
  });

  it("normalizes twitter.com status to x.com/i/status/id", () => {
    expect(normalizeSourceUrl("https://twitter.com/foo/status/1234567890123456789")).toBe(
      "https://x.com/i/status/1234567890123456789",
    );
  });

  it("returns null for non-allowlisted", () => {
    expect(normalizeSourceUrl("https://example.com")).toBeNull();
  });
});

describe("YouTube back-compat", () => {
  it("isYouTubeUrl rejects Vimeo and X", () => {
    expect(isYouTubeUrl("https://vimeo.com/123456789")).toBe(false);
    expect(isYouTubeUrl("https://x.com/u/status/1234567890123456789")).toBe(false);
    expect(isYouTubeUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toBe(true);
  });

  it("normalizeYouTubeUrl still works for YT only", () => {
    expect(normalizeYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    );
    expect(normalizeYouTubeUrl("https://vimeo.com/123")).toBeNull();
  });
});
