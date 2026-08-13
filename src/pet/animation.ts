import { useEffect, useState } from "react";
import { REMI_HANFU_CHARACTER } from "./assets";
import type { PetActivity } from "./petState";
import type { ResolvedPetPack } from "./packs";

export type PetVisualState = "idle" | "talk" | "think" | "sleep";

export const PET_FRAME_INTERVAL_MS: Record<PetVisualState, number | undefined> =
  {
    idle: 700,
    talk: 320,
    think: undefined,
    sleep: undefined,
  };

export function visualStateForActivity(activity: PetActivity): PetVisualState {
  if (activity === "sleeping") return "sleep";
  if (activity === "thinking") return "think";
  if (activity === "talking") return "talk";
  return "idle";
}

const VISUAL_PRIORITY: PetVisualState[] = ["sleep", "think", "talk", "idle"];

export function resolveVisualState(
  activeStates: readonly PetVisualState[],
): PetVisualState {
  return (
    VISUAL_PRIORITY.find((state) => activeStates.includes(state)) ?? "idle"
  );
}

export function framesForVisualState(
  state: PetVisualState,
  pack?: Pick<ResolvedPetPack, "states" | "suggestedLoops">,
): readonly string[] {
  if (pack) {
    const idle = pack.states.idle ?? REMI_HANFU_CHARACTER.states.idle;
    if (state === "idle") return pack.suggestedLoops.idle ?? idle;
    if (state === "talk") {
      return pack.suggestedLoops.talk ?? pack.states.talk ?? idle;
    }
    return pack.states[state] ?? idle;
  }
  if (state === "idle") return REMI_HANFU_CHARACTER.loops.idle;
  if (state === "talk") return REMI_HANFU_CHARACTER.loops.talk;
  return REMI_HANFU_CHARACTER.states[state];
}

export function estimateTalkDuration(text: string) {
  return Math.min(5_000, Math.max(1_200, 1_100 + text.trim().length * 45));
}

export function usePetAnimationFrame(
  state: PetVisualState,
  pack?: Pick<ResolvedPetPack, "states" | "suggestedLoops">,
) {
  const frames = framesForVisualState(state, pack);
  const [frameIndex, setFrameIndex] = useState(0);

  useEffect(() => {
    setFrameIndex(0);
    const interval = PET_FRAME_INTERVAL_MS[state];
    if (!interval || frames.length < 2) return;
    const timer = window.setInterval(
      () => setFrameIndex((current) => (current + 1) % frames.length),
      interval,
    );
    return () => window.clearInterval(timer);
  }, [frames, state]);

  return frames[frameIndex] ?? frames[0];
}
