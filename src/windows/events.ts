export const BUBBLE_USER_MESSAGE = "bubble-user-message";
export const BUBBLE_AGENT_MESSAGE = "bubble-agent-message";
export const BUBBLE_REQUEST_FINISHED = "bubble-request-finished";
export const BUBBLE_OPEN_INTERACTIVE = "bubble-open-interactive";
export const BUBBLE_SHOW_PROACTIVE = "bubble-show-proactive";
export const BUBBLE_PLACEMENT_CHANGED = "bubble-placement-changed";
export const SETTINGS_CHANGED = "settings-changed";
export const PET_STATE_CHANGED = "pet-state-changed";
export const RUN_AGENT_HEARTBEAT = "run-agent-heartbeat";
export const AGENT_HEARTBEAT_FINISHED = "agent-heartbeat-finished";
export const PROFILE_IMPORTED = "profile-imported";
export const BUBBLE_CONVERSATION_CLEARED = "bubble-conversation-cleared";

export type BubbleRequestResult = {
  ok: boolean;
  error?: string;
};

export type ProactiveBubbleEvent = {
  text: string;
  durationMs: number;
  placement: "above" | "below";
};
