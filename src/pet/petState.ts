import { invoke } from "@tauri-apps/api/core";

export type PetMood = "neutral" | "happy" | "sleepy" | "curious" | "sad";
export type PetActivity =
  "idle" | "wandering" | "sleeping" | "talking" | "thinking";

export type PetState = {
  energy: number;
  boredom: number;
  mood: PetMood;
  activity: PetActivity;
  currentGoal?: string;
  x: number;
  y: number;
  opacity: number;
  lastUserInteractionAt?: number;
  lastAgentInteractionAt?: number;
  lastHeartbeatAt?: number;
};

export type PetStatePatch = Partial<PetState>;

export function getPetState() {
  return invoke<PetState>("get_pet_state");
}

export function updatePetState(patch: PetStatePatch) {
  return invoke<PetState>("update_pet_state", { patch });
}

export function persistPetPosition(x: number, y: number) {
  return invoke<PetState>("persist_pet_position", { x, y });
}

export function recoverActivity(activity: PetActivity): PetActivity {
  return ["wandering", "talking", "thinking"].includes(activity)
    ? "idle"
    : activity;
}
