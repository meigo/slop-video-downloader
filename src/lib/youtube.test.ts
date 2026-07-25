import { describe, expect, it } from "vitest";
import { isYouTubeUrl, normalizeYouTubeUrl } from "./youtube";

describe("isYouTubeUrl", () => {
  it("accepts watch, youtu.be, shorts", () => {
    expect(isYouTubeUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toBe(true);
    expect(isYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(true);
    expect(isYouTubeUrl("https://www.youtube.com/shorts/dQw4w9WgXcQ")).toBe(true);
  });
  it("rejects non-YouTube", () => {
    expect(isYouTubeUrl("https://vimeo.com/123")).toBe(false);
    expect(isYouTubeUrl("not a url")).toBe(false);
  });
});

describe("normalizeYouTubeUrl", () => {
  it("normalizes youtu.be to watch URL", () => {
    expect(normalizeYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    );
  });
  it("returns null for non-YouTube", () => {
    expect(normalizeYouTubeUrl("https://example.com")).toBeNull();
  });
});
