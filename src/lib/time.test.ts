import { describe, expect, it } from "vitest";
import { clampRange, formatFilenameTime, formatTimestamp } from "./time";

describe("formatTimestamp", () => {
  it("formats under an hour as MM:SS", () => {
    expect(formatTimestamp(72)).toBe("01:12");
  });
  it("formats hours", () => {
    expect(formatTimestamp(3661)).toBe("1:01:01");
  });
});

describe("formatFilenameTime", () => {
  it("uses m/s tokens", () => {
    expect(formatFilenameTime(72)).toBe("01m12s");
    expect(formatFilenameTime(3661)).toBe("01h01m01s");
  });
});

describe("clampRange", () => {
  it("ensures end > start within duration", () => {
    expect(clampRange(-1, 500, 100)).toEqual({ start: 0, end: 100 });
    expect(clampRange(50, 40, 100)).toEqual({ start: 40, end: 50 });
  });
});
