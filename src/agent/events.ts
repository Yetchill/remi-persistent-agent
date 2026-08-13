export type EventType =
  | "APP_STARTED"
  | "APP_CLOSING"
  | "USER_MESSAGE"
  | "USER_CLICKED_PET"
  | "USER_DRAGGED_PET"
  | "CONVERSATION_CLEARED"
  | "BODY_HEARTBEAT"
  | "AGENT_HEARTBEAT"
  | "LONG_IDLE"
  | "ENERGY_LOW"
  | "ENERGY_RECOVERED"
  | "ACTION_FINISHED"
  | "ACTION_FAILED"
  | "MEMORY_CREATED"
  | "MEMORY_UPDATED"
  | "MEMORY_CONFLICT";

export type AgentEvent = {
  id: string;
  type: EventType;
  timestamp: number;
  payload?: unknown;
  source: "user" | "system" | "agent" | "body";
};

export function createEvent(
  type: EventType,
  source: AgentEvent["source"],
  payload?: unknown,
): AgentEvent {
  return {
    id: crypto.randomUUID(),
    type,
    timestamp: Date.now(),
    source,
    payload,
  };
}
