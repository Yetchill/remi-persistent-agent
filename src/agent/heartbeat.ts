import type { PetState } from "../pet/petState";
import type { AppSettings } from "../settings/settings";

const RECENT_USER_WINDOW_MS = 5 * 60_000;
const HOUR_MS = 60 * 60_000;

export type HeartbeatPolicyDecision = {
  allowed: boolean;
  reason: string;
};

export function canRunAgentHeartbeat(settings: AppSettings) {
  return settings.agentHeartbeat;
}

function minutesOfDay(value: string) {
  const [hours, minutes] = value.split(":").map(Number);
  if (
    !Number.isInteger(hours) ||
    !Number.isInteger(minutes) ||
    hours < 0 ||
    hours > 23 ||
    minutes < 0 ||
    minutes > 59
  ) {
    return undefined;
  }
  return hours * 60 + minutes;
}

export function isWithinQuietHours(settings: AppSettings, now: number) {
  if (!settings.quietHoursEnabled) return false;
  const start = minutesOfDay(settings.quietHoursStart);
  const end = minutesOfDay(settings.quietHoursEnd);
  if (start === undefined || end === undefined || start === end) return false;
  const date = new Date(now);
  const current = date.getHours() * 60 + date.getMinutes();
  return start < end
    ? current >= start && current < end
    : current >= start || current < end;
}

export function canSpeakProactively(
  settings: AppSettings,
  state: PetState,
  proactiveSpeechTimestamps: number[],
  now = Date.now(),
): HeartbeatPolicyDecision {
  if (!settings.proactiveInteraction) {
    return { allowed: false, reason: "proactive_interaction_off" };
  }
  if (settings.doNotDisturb) {
    return { allowed: false, reason: "do_not_disturb" };
  }
  if (isWithinQuietHours(settings, now)) {
    return { allowed: false, reason: "quiet_hours" };
  }
  if (state.activity === "sleeping") {
    return { allowed: false, reason: "agent_sleeping" };
  }
  if (
    state.lastUserInteractionAt &&
    now - state.lastUserInteractionAt < RECENT_USER_WINDOW_MS
  ) {
    return { allowed: false, reason: "recent_user_interaction" };
  }
  if (state.activity === "talking" || state.activity === "thinking") {
    return { allowed: false, reason: "agent_busy" };
  }
  const recent = proactiveSpeechTimestamps.filter(
    (timestamp) => now - timestamp < HOUR_MS,
  );
  if (recent.length >= settings.maxProactiveMessagesPerHour) {
    return { allowed: false, reason: "hourly_limit" };
  }
  const last = recent.at(-1);
  const frequencyMultiplier = {
    low: 1.5,
    normal: 1,
    high: 0.6,
  }[settings.proactiveFrequency];
  if (
    last &&
    now - last <
      settings.proactiveCooldownMinutes * frequencyMultiplier * 60_000
  ) {
    return { allowed: false, reason: "cooldown" };
  }
  return { allowed: true, reason: "policy_allows" };
}
