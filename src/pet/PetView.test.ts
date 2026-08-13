import { describe, expect, it, vi } from "vitest";
import { readPositionAfterPointerRelease } from "./PetView";

describe("PetView drag lifecycle", () => {
  it("waits for pointer release rendering before reading final position", async () => {
    const order: string[] = [];
    const waitForFrame = vi.fn(async () => {
      order.push("released");
    });
    const readPosition = vi.fn(async () => {
      order.push("position");
      return { x: 490, y: 355 };
    });

    await expect(
      readPositionAfterPointerRelease(readPosition, waitForFrame),
    ).resolves.toEqual({ x: 490, y: 355 });
    expect(order).toEqual(["released", "position"]);
  });
});
