export type BubbleMode = "hidden" | "speech" | "interactive";
export type BubblePlacement = "above" | "below";
export type BubbleSource = "proactive" | "user_conversation";

export type BubbleState = {
  mode: BubbleMode;
  text: string;
  source?: BubbleSource;
  placement: BubblePlacement;
};

export type ProactiveBubblePayload = {
  text: string;
  durationMs: number;
};

export function estimateProactiveBubbleDuration(text: string) {
  return Math.min(8_000, Math.max(3_000, 2_400 + text.trim().length * 90));
}

export function truncateSpeechText(text: string, maxCharacters = 120) {
  const normalized = text.trim();
  const characters = Array.from(normalized);
  if (characters.length <= maxCharacters) return normalized;
  return `${characters.slice(0, maxCharacters - 3).join("")}...`;
}
