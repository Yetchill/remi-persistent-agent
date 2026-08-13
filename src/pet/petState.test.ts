import { describe, expect, it } from "vitest";
import { recoverActivity } from "./petState";

describe("Pet State restart recovery", () => {
  it("resets interrupted transient activities but preserves sleep", () => {
    expect(recoverActivity("wandering")).toBe("idle");
    expect(recoverActivity("thinking")).toBe("idle");
    expect(recoverActivity("talking")).toBe("idle");
    expect(recoverActivity("sleeping")).toBe("sleeping");
  });
});
