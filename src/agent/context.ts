import type { PetState } from "../pet/petState";
import type { LlmMessage } from "../providers/types";
import type { AgentEvent } from "./events";

export type ConversationMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
};

export type ContextSources = {
  soul: string;
  soulVersion?: number;
  petState: PetState;
  relevantMemories?: string[];
  relationshipSummary?: string;
  runtimeMetadata?: RuntimeMetadata;
};

export type RuntimeMetadata = {
  activeProviderName?: string;
  activeModelDisplayName?: string;
  activeModelId?: string;
};

const ACTION_CONTRACT = `Available actions:
- {"type":"speak","text":"short natural response"}
- {"type":"remember","content":"one durable fact or useful event","sourceType":"user_explicit|agent_inferred"}
- {"type":"wander"} / {"type":"sleep"} / {"type":"wake"}
- {"type":"set_activity","activity":"idle|wandering|sleeping|talking|thinking"}
- {"type":"set_goal","goal":"short goal"}
- {"type":"set_mood","mood":"neutral|happy|sleepy|curious|sad"}
- {"type":"noop"}
Use remember only for stable preferences/facts, meaningful time-bound events, or shared relationship facts. Never remember greetings or routine small talk.
Use user_explicit only when the user directly stated the content; otherwise use agent_inferred.
Respond only with {"intent":"short_label","actions":[...]}. Do not use markdown or reveal chain-of-thought.
For every user message, normally include one short, natural speak action.`;

export function buildContext(
  event: AgentEvent,
  recentConversation: ConversationMessage[],
  sources: ContextSources,
): LlmMessage[] {
  const recent = recentConversation.slice(-12).map<LlmMessage>((message) => ({
    role: message.role,
    content: message.content,
  }));
  const systemSections = [
    `[SOUL]\n${sources.soul}`,
    `[SOUL VERSION]\n${sources.soulVersion ?? 1}`,
    `[AGENT IDENTITY]\nThe persistent Agent identity is defined by SOUL. The Agent is Remi; the backend model is only a replaceable reasoning engine.`,
    `[RUNTIME BACKEND]\nProvider = ${sources.runtimeMetadata?.activeProviderName ?? "Not configured"}\nModel = ${sources.runtimeMetadata?.activeModelDisplayName ?? "Not configured"}\nModel ID = ${sources.runtimeMetadata?.activeModelId ?? "Not configured"}`,
    `[CURRENT STATE]\n${JSON.stringify(sources.petState)}`,
    `[CURRENT EVENT]\n${event.type}: ${JSON.stringify(event.payload ?? {})}`,
    `[MEMORY POLICY]\nevolving-memory-v1; retrieve active memories only; remember actions become candidates and never bypass lifecycle validation.`,
  ];
  if (sources.relevantMemories?.length) {
    systemSections.push(
      `[RELEVANT MEMORIES]\n${sources.relevantMemories.join("\n")}`,
    );
  }
  if (sources.relationshipSummary) {
    systemSections.push(
      `[RELATIONSHIP SUMMARY]\n${sources.relationshipSummary}`,
    );
  }
  systemSections.push(
    `[AVAILABLE ACTIONS + OUTPUT CONTRACT]\n${ACTION_CONTRACT}`,
  );
  if (event.type === "AGENT_HEARTBEAT") {
    systemSections.push(
      `[HEARTBEAT POLICY]\nChoose only a high-level action: noop, wander, sleep, or one brief speak. Prefer noop. Do not invent user facts. Local policy may reject proactive speech.`,
    );
  }

  return [{ role: "system", content: systemSections.join("\n\n") }, ...recent];
}
