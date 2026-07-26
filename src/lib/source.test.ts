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

  it("rejects unsupported and invalid", () => {
    expect(isSupportedUrl("https://instagram.com/p/abc")).toBe(false);
    expect(isSupportedUrl("https://vimeo.com/")).toBe(false);
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

  it("returns null for non-allowlisted", () => {
    expect(normalizeSourceUrl("https://example.com")).toBeNull();
  });
});

describe("YouTube back-compat", () => {
  it("isYouTubeUrl rejects Vimeo", () => {
    expect(isYouTubeUrl("https://vimeo.com/123456789")).toBe(false);
    expect(isYouTubeUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toBe(true);
  });

  it("normalizeYouTubeUrl still works for YT only", () => {
    expect(normalizeYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    );
    expect(normalizeYouTubeUrl("https://vimeo.com/123")).toBeNull();
  });
});
