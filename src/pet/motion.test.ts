import { describe, expect, it, vi } from "vitest";
import {
  interpolatePosition,
  legalPositionBounds,
  randomValidPosition,
  runWanderLoop,
} from "./motion";
import type { WorkArea } from "./screen";

const workArea: WorkArea = {
  x: -1440,
  y: 25,
  width: 1440,
  height: 875,
  scaleFactor: 2,
  petWidth: 320,
  petHeight: 320,
};

describe("local motion engine", () => {
  it("uses the physical window size reported by Tauri", () => {
    expect(legalPositionBounds(workArea)).toEqual({
      minX: -1440,
      maxX: -320,
      minY: 25,
      maxY: 580,
    });
  });

  it("accounts for work-area origin and pet size", () => {
    expect(legalPositionBounds(workArea, { width: 160, height: 160 })).toEqual({
      minX: -1440,
      maxX: -160,
      minY: 25,
      maxY: 740,
    });
  });

  it("always chooses a position inside the legal area", () => {
    expect(
      randomValidPosition(workArea, { width: 160, height: 160 }, () => 0),
    ).toEqual({
      x: -1440,
      y: 25,
    });
    expect(
      randomValidPosition(workArea, { width: 160, height: 160 }, () => 1),
    ).toEqual({
      x: -160,
      y: 740,
    });
  });

  it("smoothly interpolates and clamps progress", () => {
    expect(
      interpolatePosition({ x: 10, y: 20 }, { x: 110, y: 220 }, -1),
    ).toEqual({ x: 10, y: 20 });
    expect(
      interpolatePosition({ x: 10, y: 20 }, { x: 110, y: 220 }, 0.5),
    ).toEqual({ x: 60, y: 120 });
    expect(
      interpolatePosition({ x: 10, y: 20 }, { x: 110, y: 220 }, 2),
    ).toEqual({ x: 110, y: 220 });
  });

  it("rechecks Auto Wander before starting a delayed move", async () => {
    vi.useFakeTimers();
    const move = vi.fn().mockResolvedValue(undefined);
    const abortController = new AbortController();
    const loop = runWanderLoop({
      workArea,
      initialPosition: { x: -1000, y: 100 },
      move,
      signal: abortController.signal,
      wanderDelayMs: 100,
      canMove: vi.fn().mockResolvedValue(false),
    });
    await vi.advanceTimersByTimeAsync(250);
    abortController.abort();
    await loop;
    expect(move).not.toHaveBeenCalled();
    vi.useRealTimers();
  });
});
