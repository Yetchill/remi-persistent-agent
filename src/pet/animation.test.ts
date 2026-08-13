import { describe, expect, it } from "vitest";
import {
  estimateTalkDuration,
  framesForVisualState,
  resolveVisualState,
  visualStateForActivity,
} from "./animation";

describe("Remi Hanfu visual animation", () => {
  it("uses manifest loops for idle and talk and static frames for think/sleep", () => {
    expect(framesForVisualState("idle")).toHaveLength(4);
    expect(framesForVisualState("talk")).toHaveLength(4);
    expect(framesForVisualState("think")).toHaveLength(1);
    expect(framesForVisualState("sleep")).toHaveLength(1);
    expect(framesForVisualState("idle")[1]).toBe(
      framesForVisualState("idle")[3],
    );
    expect(framesForVisualState("talk")[1]).toBe(
      framesForVisualState("talk")[3],
    );
  });

  it("maps persisted Agent activity without creating another Agent state", () => {
    expect(visualStateForActivity("idle")).toBe("idle");
    expect(visualStateForActivity("wandering")).toBe("idle");
    expect(visualStateForActivity("thinking")).toBe("think");
    expect(visualStateForActivity("talking")).toBe("talk");
    expect(visualStateForActivity("sleeping")).toBe("sleep");
  });

  it("resolves simultaneous visual signals with the documented priority", () => {
    expect(resolveVisualState(["idle", "talk"])).toBe("talk");
    expect(resolveVisualState(["talk", "think"])).toBe("think");
    expect(resolveVisualState(["think", "sleep"])).toBe("sleep");
  });

  it("uses deterministic bounded talk timing", () => {
    expect(estimateTalkDuration("好")).toBe(1_200);
    expect(estimateTalkDuration("a".repeat(1_000))).toBe(5_000);
    expect(estimateTalkDuration("这是一条稍长的回复")).toBeGreaterThan(1_200);
  });
});
