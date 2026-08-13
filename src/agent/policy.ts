import type { AgentEvent } from "./events";

export function requiresLlm(event: AgentEvent) {
  return event.type === "USER_MESSAGE" || event.type === "AGENT_HEARTBEAT";
}
