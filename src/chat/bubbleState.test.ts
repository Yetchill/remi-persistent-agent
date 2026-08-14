import { describe, expect, it } from "vitest";
import {
  estimateSpeechBubbleDuration,
  truncateSpeechText,
} from "./bubbleState";

describe("proactive bubble timing", () => {
  it("keeps short and long Unicode messages inside the display bounds", () => {
    expect(
      estimateSpeechBubbleDuration("辛苦啦 (｡･ω･｡) ✨"),
    ).toBeGreaterThanOrEqual(3_000);
    expect(estimateSpeechBubbleDuration("a".repeat(1_000))).toBe(8_000);
  });

  it("limits proactive speech without breaking Unicode rendering", () => {
    expect(truncateSpeechText("早呀～ ✨")).toBe("早呀～ ✨");
    const truncated = truncateSpeechText("猫".repeat(130));
    expect(Array.from(truncated)).toHaveLength(120);
    expect(truncated.endsWith("...")).toBe(true);
  });
});
