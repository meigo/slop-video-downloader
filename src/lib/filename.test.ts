import { describe, expect, it } from "vitest";
import { buildClipFilename, sanitizeTitle } from "./filename";

describe("sanitizeTitle", () => {
  it("strips unsafe characters", () => {
    expect(sanitizeTitle('My Video: "Cool"/Thing?')).toBe("My Video Cool Thing");
  });
  it("collapses whitespace and trims", () => {
    expect(sanitizeTitle("  a   b  ")).toBe("a b");
  });
});

describe("buildClipFilename", () => {
  it("builds title_start-end.mp4", () => {
    expect(buildClipFilename("Hello World", 72, 105)).toBe(
      "Hello World_01m12s-01m45s.mp4",
    );
  });
  it("supports audio extension", () => {
    expect(buildClipFilename("Hello World", 72, 105, "m4a")).toBe(
      "Hello World_01m12s-01m45s.m4a",
    );
  });
});
