export type BubbleMode = "hidden" | "speech" | "interactive";
export type BubblePlacement = "above" | "below" | "left" | "right";
export type BubbleSource = "proactive" | "user_conversation";

export type BubbleState = {
  mode: BubbleMode;
  text: string;
  source?: BubbleSource;
  placement: BubblePlacement;
};

export type SpeechBubblePayload = {
  text: string;
  durationMs: number;
  source: BubbleSource;
  placement: BubblePlacement;
};

export function estimateSpeechBubbleDuration(text: string) {
  const characterCount = Array.from(text.trim()).length;
  return Math.min(20_000, Math.max(3_200, 2_600 + characterCount * 85));
}

export function truncateSpeechText(text: string, maxCharacters = 120) {
  const normalized = text.trim();
  const characters = Array.from(normalized);
  if (characters.length <= maxCharacters) return normalized;
  return `${characters.slice(0, maxCharacters - 3).join("")}...`;
}
