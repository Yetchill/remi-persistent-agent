import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS } from "../settings/settings";
import type { PetState } from "../pet/petState";
import { canSpeakProactively } from "./heartbeat";

const state: PetState = {
  energy: 80,
  boredom: 20,
  mood: "neutral",
  activity: "idle",
  x: 0,
  y: 0,
  opacity: 1,
};

describe("Agent Heartbeat proactive policy", () => {
  it("absolutely blocks speech while proactive interaction is off", () => {
    expect(canSpeakProactively(DEFAULT_SETTINGS, state, [], 10_000)).toEqual({
      allowed: false,
      reason: "proactive_interaction_off",
    });
  });

  it("enforces cooldown and hourly limits", () => {
    const settings = {
      ...DEFAULT_SETTINGS,
      proactiveInteraction: true,
      maxProactiveMessagesPerHour: 2,
      proactiveCooldownMinutes: 30,
    };
    expect(canSpeakProactively(settings, state, [9_000], 10_000).reason).toBe(
      "cooldown",
    );
    expect(
      canSpeakProactively(settings, state, [1_000, 2_000], 10_000).reason,
    ).toBe("hourly_limit");
  });

  it("blocks DND and overnight quiet hours without affecting user chat", () => {
    const atMidnight = new Date(2026, 0, 1, 0, 30).getTime();
    expect(
      canSpeakProactively(
        { ...DEFAULT_SETTINGS, proactiveInteraction: true, doNotDisturb: true },
        state,
        [],
        atMidnight,
      ).reason,
    ).toBe("do_not_disturb");
    expect(
      canSpeakProactively(
        {
          ...DEFAULT_SETTINGS,
          proactiveInteraction: true,
          quietHoursEnabled: true,
          quietHoursStart: "23:00",
          quietHoursEnd: "08:00",
        },
        state,
        [],
        atMidnight,
      ).reason,
    ).toBe("quiet_hours");
  });

  it("blocks proactive speech while the pet is sleeping", () => {
    expect(
      canSpeakProactively(
        { ...DEFAULT_SETTINGS, proactiveInteraction: true },
        { ...state, activity: "sleeping" },
        [],
        10_000,
      ),
    ).toEqual({ allowed: false, reason: "agent_sleeping" });
  });
});
