import { invoke } from "@tauri-apps/api/core";
import type { PetState } from "../pet/petState";

export type PetSize = "small" | "medium" | "large";
export type MovementSpeed = "slow" | "normal" | "fast";
export type ProactiveFrequency = "low" | "normal" | "high";

export type AppSettings = {
  petName: string;
  petSize: PetSize;
  autoWander: boolean;
  wanderIntervalSeconds: number;
  movementSpeed: MovementSpeed;
  proactiveInteraction: boolean;
  proactiveFrequency: ProactiveFrequency;
  agentHeartbeat: boolean;
  agentHeartbeatIntervalSeconds: number;
  proactiveCooldownMinutes: number;
  maxProactiveMessagesPerHour: number;
  doNotDisturb: boolean;
  quietHoursEnabled: boolean;
  quietHoursStart: string;
  quietHoursEnd: string;
};

export type RuntimeOverview = {
  petState: PetState;
  memoryCount: number;
  semanticCount: number;
  episodicCount: number;
  relationshipCount: number;
  traceCount: number;
  lastProactiveAction?: {
    actionType: string;
    timestamp: number;
    payloadJson?: string;
    success: boolean;
    reason?: string;
  };
};

export const DEFAULT_SETTINGS: AppSettings = {
  petName: "Remi",
  petSize: "large",
  autoWander: true,
  wanderIntervalSeconds: 30,
  movementSpeed: "normal",
  proactiveInteraction: false,
  proactiveFrequency: "normal",
  agentHeartbeat: false,
  agentHeartbeatIntervalSeconds: 60,
  proactiveCooldownMinutes: 30,
  maxProactiveMessagesPerHour: 2,
  doNotDisturb: false,
  quietHoursEnabled: false,
  quietHoursStart: "23:00",
  quietHoursEnd: "08:00",
};

export function getAppSettings() {
  return invoke<AppSettings>("get_app_settings");
}

export function updateAppSettings(settings: AppSettings) {
  return invoke<AppSettings>("update_app_settings", { settings });
}

export function getRuntimeOverview() {
  return invoke<RuntimeOverview>("get_runtime_overview");
}

export function movementDuration(speed: MovementSpeed) {
  return { slow: 4_000, normal: 2_400, fast: 1_400 }[speed];
}
