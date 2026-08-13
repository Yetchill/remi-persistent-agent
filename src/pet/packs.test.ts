import { describe, expect, it } from "vitest";
import { frameFilesForState, type PetPack } from "./packs";

const minimalPack: PetPack = {
  id: "minimal",
  name: "Minimal",
  version: "1.0",
  defaultState: "idle",
  states: { idle: ["idle.png"] },
  suggestedLoops: {},
  source: "imported",
  rootPath: "/tmp/minimal",
};

describe("Pet Pack state fallback", () => {
  it("uses idle when an optional visual state is absent", () => {
    expect(frameFilesForState(minimalPack, "talk")).toEqual(["idle.png"]);
    expect(frameFilesForState(minimalPack, "think")).toEqual(["idle.png"]);
    expect(frameFilesForState(minimalPack, "sleep")).toEqual(["idle.png"]);
  });

  it("prefers a suggested animation loop when present", () => {
    expect(
      frameFilesForState(
        {
          ...minimalPack,
          suggestedLoops: { idle: ["idle.png", "idle.png"] },
        },
        "idle",
      ),
    ).toHaveLength(2);
  });
});
