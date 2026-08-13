import { invoke } from "@tauri-apps/api/core";
import { consolidateMemories, writeMemory } from "../memory/manager";
import type { MemoryWriteResult } from "../memory/types";
import { getRelationshipSummary, retrieveMemories } from "../memory/retrieval";
import {
  getRecentWorkingMessages,
  persistWorkingMessage,
} from "../memory/working";
import {
  getPetState,
  updatePetState,
  type PetStatePatch,
} from "../pet/petState";
import { OpenAiCompatibleProvider } from "../providers/llm/openaiCompatible";
import { getProviderCatalog } from "../providers/config";
import type { LlmProvider } from "../providers/types";
import { getSoul } from "../soul/soul";
import { getAppSettings } from "../settings/settings";
import type { AppSettings } from "../settings/settings";
import {
  AgentResponseValidationError,
  parseAgentResponse,
  type AgentAction,
} from "./actions";
import {
  buildContext,
  type ContextSources,
  type ConversationMessage,
} from "./context";
import { createEvent, type AgentEvent } from "./events";
import { requiresLlm } from "./policy";
import { canSpeakProactively } from "./heartbeat";

type TraceActionInput = {
  id: string;
  eventId: string;
  actionType: string;
  timestamp: number;
  payloadJson?: string;
  success: boolean;
  error?: string;
};

export type RuntimeDependencies = {
  provider: LlmProvider;
  traceEvent: (event: AgentEvent) => Promise<unknown>;
  traceAction: (trace: TraceActionInput) => Promise<unknown>;
  loadContextSources: (event: AgentEvent) => Promise<ContextSources>;
  loadRecentMessages: () => Promise<ConversationMessage[]>;
  persistMessage: (message: ConversationMessage) => Promise<unknown>;
  writeMemory: (
    content: string,
    sourceRef: string,
    sourceType:
      | "user_explicit"
      | "agent_inferred"
      | "conversation"
      | "heartbeat"
      | "reflection"
      | "system",
  ) => Promise<MemoryWriteResult | undefined>;
  canAutoWander: () => Promise<boolean>;
  consolidateMemories?: () => Promise<number>;
  loadHeartbeatPolicy?: () => Promise<{
    settings: AppSettings;
    petState: Awaited<ReturnType<typeof getPetState>>;
  }>;
  updatePetState: (patch: PetStatePatch) => Promise<unknown>;
  onMessage: (message: ConversationMessage, event?: AgentEvent) => void;
  onActionExecuted?: (
    event: AgentEvent,
    action: AgentAction,
  ) => void | Promise<void>;
};

export class AgentRuntime {
  private readonly recentConversation: ConversationMessage[] = [];
  private hydration?: Promise<void>;
  private appStartedRecorded = false;
  private appClosingRecorded = false;
  private proactiveSpeechTimestamps: number[] = [];

  constructor(private readonly dependencies: RuntimeDependencies) {}

  static createDesktopRuntime(
    onMessage: RuntimeDependencies["onMessage"],
    onActionExecuted?: RuntimeDependencies["onActionExecuted"],
  ) {
    return new AgentRuntime({
      provider: new OpenAiCompatibleProvider(),
      traceEvent: (event) =>
        invoke("trace_event", {
          trace: {
            id: event.id,
            eventType: event.type,
            source: event.source,
            timestamp: event.timestamp,
            payloadJson:
              event.payload === undefined
                ? undefined
                : JSON.stringify(event.payload),
          },
        }),
      traceAction: (trace) => invoke("trace_action", { trace }),
      loadContextSources: async (event) => {
        const query =
          event.payload &&
          typeof event.payload === "object" &&
          "text" in event.payload
            ? String(event.payload.text)
            : event.type;
        const [soul, petState, memories, relationshipSummary, catalog] =
          await Promise.all([
            getSoul(),
            getPetState(),
            retrieveMemories(query, 6),
            getRelationshipSummary(),
            getProviderCatalog(),
          ]);
        const activeProvider = catalog.providers.find(
          (provider) => provider.id === catalog.activeProviderId,
        );
        const activeModel = activeProvider?.models.find(
          (model) => model.id === catalog.activeModelId,
        );
        return {
          soul: soul.content,
          soulVersion: soul.soulVersion,
          petState,
          relevantMemories: memories.map(
            (memory) => `[${memory.kind}] ${memory.content}`,
          ),
          relationshipSummary,
          runtimeMetadata: {
            activeProviderName: activeProvider?.displayName,
            activeModelDisplayName: activeModel?.displayName,
            activeModelId: activeModel?.modelId,
          },
        };
      },
      loadRecentMessages: () => getRecentWorkingMessages(50),
      persistMessage: persistWorkingMessage,
      writeMemory,
      canAutoWander: async () => (await getAppSettings()).autoWander,
      consolidateMemories,
      loadHeartbeatPolicy: async () => ({
        settings: await getAppSettings(),
        petState: await getPetState(),
      }),
      updatePetState,
      onMessage,
      onActionExecuted,
    });
  }

  async recordAppStarted() {
    if (this.appStartedRecorded) return;
    this.appStartedRecorded = true;
    await this.ensureHydrated();
    await this.handleEvent(createEvent("APP_STARTED", "system"));
  }

  async recordPetClick() {
    await this.ensureHydrated();
    await this.dependencies.updatePetState({
      lastUserInteractionAt: Date.now(),
    });
    await this.handleEvent(createEvent("USER_CLICKED_PET", "user"));
  }

  async recordAppClosing() {
    if (this.appClosingRecorded) return;
    this.appClosingRecorded = true;
    await this.handleEvent(createEvent("APP_CLOSING", "system"));
    await this.dependencies.consolidateMemories?.();
  }

  async recordPetDrag(position: { x: number; y: number }) {
    await this.handleEvent(createEvent("USER_DRAGGED_PET", "user", position));
  }

  async recordBodyHeartbeat() {
    await this.handleEvent(createEvent("BODY_HEARTBEAT", "body"));
  }

  async clearRecentConversation() {
    this.recentConversation.length = 0;
    await this.dependencies.traceEvent(
      createEvent("CONVERSATION_CLEARED", "user", {
        scope: "working_memory_only",
      }),
    );
  }

  async recordAgentHeartbeat() {
    await this.ensureHydrated();
    await this.handleEvent(createEvent("AGENT_HEARTBEAT", "agent"));
  }

  async handleUserMessage(text: string) {
    const content = text.trim();
    if (!content) return;
    await this.ensureHydrated();
    const message: ConversationMessage = {
      id: crypto.randomUUID(),
      role: "user",
      content,
      timestamp: Date.now(),
    };
    await this.appendMessage(message);
    await this.dependencies.updatePetState({
      activity: "thinking",
      lastUserInteractionAt: message.timestamp,
    });
    try {
      await this.handleEvent(
        createEvent("USER_MESSAGE", "user", { text: content }),
      );
    } finally {
      await this.dependencies.updatePetState({ activity: "idle" });
    }
  }

  async handleEvent(event: AgentEvent) {
    await this.dependencies.traceEvent(event);
    if (!requiresLlm(event)) return;

    let sources: ContextSources | undefined;
    try {
      sources = await this.dependencies.loadContextSources(event);
      const response = await this.dependencies.provider.complete({
        eventId: event.id,
        eventType: event.type,
        messages: buildContext(event, this.recentConversation, sources),
        traceMetadata: {
          provider: sources.runtimeMetadata?.activeProviderName,
          model: sources.runtimeMetadata?.activeModelId,
          soulVersion: sources.soulVersion ?? 1,
          memoryPolicyVersion: "evolving-memory-v1",
        },
      });
      const parsed = parseAgentResponse(response.content);
      if (parsed.fallbackUsed) {
        await this.dependencies.traceAction({
          id: crypto.randomUUID(),
          eventId: event.id,
          actionType: "response_parse_fallback",
          timestamp: Date.now(),
          payloadJson: JSON.stringify({
            provider: sources.runtimeMetadata?.activeProviderName,
            model: sources.runtimeMetadata?.activeModelDisplayName,
            modelId: sources.runtimeMetadata?.activeModelId,
            parseStage: parsed.stage,
            fallbackUsed: true,
          }),
          success: false,
          error: parsed.parseError,
        });
      }
      for (const action of parsed.envelope.actions) {
        await this.executeAction(event, action);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const validationFailure = error instanceof AgentResponseValidationError;
      await this.dependencies.traceAction({
        id: crypto.randomUUID(),
        eventId: event.id,
        actionType: validationFailure
          ? "invalid_agent_action"
          : "invalid_or_failed_response",
        timestamp: Date.now(),
        payloadJson: JSON.stringify({
          provider: sources?.runtimeMetadata?.activeProviderName,
          model: sources?.runtimeMetadata?.activeModelDisplayName,
          modelId: sources?.runtimeMetadata?.activeModelId,
          parseStage: validationFailure ? "validation" : "provider_or_runtime",
          fallbackUsed: false,
        }),
        success: false,
        error: message,
      });
      await this.dependencies.traceEvent(
        createEvent("ACTION_FAILED", "agent", {
          parentEventId: event.id,
          reason: message,
        }),
      );
      if (validationFailure) return;
      throw error;
    }
  }

  private ensureHydrated() {
    this.hydration ??= this.dependencies
      .loadRecentMessages()
      .then((messages) => {
        for (const message of messages) {
          if (
            this.recentConversation.some(
              (existing) => existing.id === message.id,
            )
          )
            continue;
          this.recentConversation.push(message);
          if (this.recentConversation.length > 12)
            this.recentConversation.shift();
          this.dependencies.onMessage(message);
        }
      });
    return this.hydration;
  }

  private async appendMessage(
    message: ConversationMessage,
    event?: AgentEvent,
  ) {
    await this.dependencies.persistMessage(message);
    this.recentConversation.push(message);
    if (this.recentConversation.length > 12) this.recentConversation.shift();
    this.dependencies.onMessage(message, event);
  }

  private async executeAction(event: AgentEvent, action: AgentAction) {
    const now = Date.now();
    switch (action.type) {
      case "speak":
        if (event.type === "AGENT_HEARTBEAT") {
          const policy = this.dependencies.loadHeartbeatPolicy
            ? await this.dependencies.loadHeartbeatPolicy()
            : undefined;
          const decision = policy
            ? canSpeakProactively(
                policy.settings,
                policy.petState,
                this.proactiveSpeechTimestamps,
                now,
              )
            : { allowed: false, reason: "heartbeat_policy_unavailable" };
          if (!decision.allowed) {
            await this.dependencies.traceAction({
              id: crypto.randomUUID(),
              eventId: event.id,
              actionType: "speak_blocked",
              timestamp: now,
              payloadJson: JSON.stringify({ reason: decision.reason }),
              success: false,
              error: decision.reason,
            });
            await this.dependencies.traceEvent(
              createEvent("ACTION_FAILED", "agent", {
                parentEventId: event.id,
                action: "speak",
                reason: decision.reason,
              }),
            );
            return;
          }
          this.proactiveSpeechTimestamps = this.proactiveSpeechTimestamps
            .filter((timestamp) => now - timestamp < 60 * 60_000)
            .concat(now);
        }
        await this.appendMessage(
          {
            id: crypto.randomUUID(),
            role: "assistant",
            content: action.text,
            timestamp: now,
          },
          event,
        );
        await this.dependencies.updatePetState({
          activity: "talking",
          lastAgentInteractionAt: now,
        });
        break;
      case "remember":
        {
          const result = await this.dependencies.writeMemory(
            action.content,
            event.id,
            event.type === "AGENT_HEARTBEAT"
              ? "heartbeat"
              : (action.sourceType ??
                  (event.type === "USER_MESSAGE"
                    ? "conversation"
                    : "agent_inferred")),
          );
          if (result) {
            await this.dependencies.traceEvent(
              createEvent(
                result.decision === "ADD"
                  ? "MEMORY_CREATED"
                  : result.decision === "SUPERSEDE"
                    ? "MEMORY_CONFLICT"
                    : "MEMORY_UPDATED",
                "agent",
                {
                  parentEventId: event.id,
                  memoryId: result.memory?.id,
                  operation: result.decision,
                  reasonLabel: result.lifecycle?.reasonLabel,
                },
              ),
            );
          }
        }
        break;
      case "wander":
        if (await this.dependencies.canAutoWander()) {
          await this.dependencies.updatePetState({ activity: "wandering" });
        }
        break;
      case "sleep":
        await this.dependencies.updatePetState({
          activity: "sleeping",
          mood: "sleepy",
        });
        break;
      case "wake":
        await this.dependencies.updatePetState({
          activity: "idle",
          mood: "neutral",
        });
        break;
      case "set_activity":
        await this.dependencies.updatePetState({ activity: action.activity });
        break;
      case "set_goal":
        await this.dependencies.updatePetState({ currentGoal: action.goal });
        break;
      case "set_mood":
        await this.dependencies.updatePetState({ mood: action.mood });
        break;
      case "noop":
        break;
    }
    await this.dependencies.traceAction({
      id: crypto.randomUUID(),
      eventId: event.id,
      actionType: action.type,
      timestamp: now,
      payloadJson: JSON.stringify(action),
      success: true,
    });
    await this.dependencies.traceEvent(
      createEvent("ACTION_FINISHED", "agent", {
        parentEventId: event.id,
        action: action.type,
      }),
    );
    await this.dependencies.onActionExecuted?.(event, action);
  }
}
