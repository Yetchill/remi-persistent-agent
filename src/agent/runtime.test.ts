import { describe, expect, it, vi } from "vitest";
import { DEFAULT_SETTINGS } from "../settings/settings";
import { AgentRuntime } from "./runtime";

const loadContextSources = vi.fn().mockResolvedValue({
  soul: "Name: Remi",
  runtimeMetadata: {
    activeProviderName: "DeepSeek",
    activeModelDisplayName: "DeepSeek Reasoner",
    activeModelId: "deepseek-reasoner",
  },
  petState: {
    energy: 100,
    boredom: 0,
    mood: "neutral",
    activity: "idle",
    x: 0,
    y: 0,
    opacity: 1,
  },
});

describe("AgentRuntime chat route", () => {
  it("routes a user message through event, context, provider, validator and executor", async () => {
    const provider = {
      complete: vi.fn().mockResolvedValue({
        content: JSON.stringify({
          intent: "reply",
          actions: [
            { type: "speak", text: "Hello from runtime" },
            {
              type: "remember",
              content: "User prefers concise replies",
              sourceType: "user_explicit",
            },
          ],
        }),
      }),
    };
    const traceEvent = vi.fn().mockResolvedValue(undefined);
    const traceAction = vi.fn().mockResolvedValue(undefined);
    const updatePetState = vi.fn().mockResolvedValue(undefined);
    const persistMessage = vi.fn().mockResolvedValue(undefined);
    const writeMemory = vi.fn().mockResolvedValue({ decision: "ADD" });
    const onMessage = vi.fn();
    const runtime = new AgentRuntime({
      provider,
      traceEvent,
      traceAction,
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage,
      writeMemory,
      canAutoWander: vi.fn().mockResolvedValue(true),
      updatePetState,
      onMessage,
    });

    await runtime.handleUserMessage("Hi");

    expect(traceEvent).toHaveBeenCalledWith(
      expect.objectContaining({ type: "USER_MESSAGE" }),
    );
    expect(provider.complete).toHaveBeenCalledOnce();
    expect(provider.complete.mock.calls[0][0].messages[0].role).toBe("system");
    expect(provider.complete.mock.calls[0][0].messages[0].content).toContain(
      "Name: Remi",
    );
    expect(traceAction).toHaveBeenCalledWith(
      expect.objectContaining({ actionType: "speak" }),
    );
    expect(writeMemory).toHaveBeenCalledWith(
      "User prefers concise replies",
      expect.any(String),
      "user_explicit",
    );
    expect(persistMessage).toHaveBeenCalledTimes(2);
    expect(onMessage).toHaveBeenLastCalledWith(
      expect.objectContaining({
        role: "assistant",
        content: "Hello from runtime",
      }),
      expect.objectContaining({ type: "USER_MESSAGE" }),
    );
  });

  it("traces Body Heartbeat without calling the LLM", async () => {
    const provider = { complete: vi.fn() };
    const traceEvent = vi.fn().mockResolvedValue(undefined);
    const runtime = new AgentRuntime({
      provider,
      traceEvent,
      traceAction: vi.fn().mockResolvedValue(undefined),
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(true),
      updatePetState: vi.fn().mockResolvedValue(undefined),
      onMessage: vi.fn(),
    });

    await runtime.recordBodyHeartbeat();
    await runtime.recordAppClosing();
    await runtime.recordAppClosing();

    expect(traceEvent).toHaveBeenCalledWith(
      expect.objectContaining({ type: "BODY_HEARTBEAT" }),
    );
    expect(provider.complete).not.toHaveBeenCalled();
    expect(
      traceEvent.mock.calls.filter(([event]) => event.type === "APP_CLOSING"),
    ).toHaveLength(1);
  });

  it("notifies the UI bridge only after a validated action executes", async () => {
    const onActionExecuted = vi.fn();
    const runtime = new AgentRuntime({
      provider: {
        complete: vi.fn().mockResolvedValue({
          content: JSON.stringify({
            actions: [{ type: "speak", text: "Bridge reply ✨" }],
          }),
        }),
      },
      traceEvent: vi.fn().mockResolvedValue(undefined),
      traceAction: vi.fn().mockResolvedValue(undefined),
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(true),
      updatePetState: vi.fn().mockResolvedValue(undefined),
      onMessage: vi.fn(),
      onActionExecuted,
    });

    await runtime.handleUserMessage("hello");

    expect(onActionExecuted).toHaveBeenCalledWith(
      expect.objectContaining({ type: "USER_MESSAGE" }),
      { type: "speak", text: "Bridge reply ✨" },
    );
  });

  it("blocks Agent Heartbeat speech when proactive interaction is off", async () => {
    const onMessage = vi.fn();
    const traceAction = vi.fn().mockResolvedValue(undefined);
    const runtime = new AgentRuntime({
      provider: {
        complete: vi.fn().mockResolvedValue({
          content: JSON.stringify({
            actions: [{ type: "speak", text: "Unrequested message" }],
          }),
        }),
      },
      traceEvent: vi.fn().mockResolvedValue(undefined),
      traceAction,
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(true),
      loadHeartbeatPolicy: vi.fn().mockResolvedValue({
        settings: {
          ...DEFAULT_SETTINGS,
          agentHeartbeat: true,
          proactiveInteraction: false,
        },
        petState: {
          energy: 100,
          boredom: 0,
          mood: "neutral",
          activity: "idle",
          x: 0,
          y: 0,
          opacity: 1,
        },
      }),
      updatePetState: vi.fn().mockResolvedValue(undefined),
      onMessage,
    });

    await runtime.recordAgentHeartbeat();

    expect(onMessage).not.toHaveBeenCalledWith(
      expect.objectContaining({ content: "Unrequested message" }),
    );
    expect(traceAction).toHaveBeenCalledWith(
      expect.objectContaining({ actionType: "speak_blocked" }),
    );
  });

  it("rejects actions outside the whitelist without exposing parser errors", async () => {
    const traceAction = vi.fn().mockResolvedValue(undefined);
    const runtime = new AgentRuntime({
      provider: {
        complete: vi.fn().mockResolvedValue({
          content: JSON.stringify({
            actions: [{ type: "shell", command: "whoami" }],
          }),
        }),
      },
      traceEvent: vi.fn().mockResolvedValue(undefined),
      traceAction,
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(true),
      updatePetState: vi.fn().mockResolvedValue(undefined),
      onMessage: vi.fn(),
    });

    await expect(
      runtime.handleUserMessage("run this"),
    ).resolves.toBeUndefined();
    expect(traceAction).toHaveBeenCalledWith(
      expect.objectContaining({
        actionType: "invalid_agent_action",
        success: false,
      }),
    );
  });

  it("hydrates persisted Working Memory before building context", async () => {
    const provider = {
      complete: vi.fn().mockResolvedValue({
        content: JSON.stringify({
          actions: [{ type: "speak", text: "Still here" }],
        }),
      }),
    };
    const onMessage = vi.fn();
    const runtime = new AgentRuntime({
      provider,
      traceEvent: vi.fn().mockResolvedValue(undefined),
      traceAction: vi.fn().mockResolvedValue(undefined),
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([
        {
          id: "persisted-1",
          role: "user",
          content: "Remember this prior turn",
          timestamp: 1,
        },
      ]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(true),
      updatePetState: vi.fn().mockResolvedValue(undefined),
      onMessage,
    });

    await runtime.handleUserMessage("New turn");

    expect(provider.complete.mock.calls[0][0].messages).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ content: "Remember this prior turn" }),
      ]),
    );
    expect(onMessage).toHaveBeenCalledWith(
      expect.objectContaining({ id: "persisted-1" }),
    );
  });

  it("does not let a wander action bypass Auto Wander OFF", async () => {
    const updatePetState = vi.fn().mockResolvedValue(undefined);
    const runtime = new AgentRuntime({
      provider: {
        complete: vi.fn().mockResolvedValue({
          content: JSON.stringify({ actions: [{ type: "wander" }] }),
        }),
      },
      traceEvent: vi.fn().mockResolvedValue(undefined),
      traceAction: vi.fn().mockResolvedValue(undefined),
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(false),
      updatePetState,
      onMessage: vi.fn(),
    });

    await runtime.handleUserMessage("Stay here");

    expect(updatePetState).not.toHaveBeenCalledWith({ activity: "wandering" });
  });

  it("turns plain text into safe speak and traces the parser fallback", async () => {
    const traceAction = vi.fn().mockResolvedValue(undefined);
    const onMessage = vi.fn();
    const runtime = new AgentRuntime({
      provider: {
        complete: vi.fn().mockResolvedValue({ content: "我是 Remi。" }),
      },
      traceEvent: vi.fn().mockResolvedValue(undefined),
      traceAction,
      loadContextSources,
      loadRecentMessages: vi.fn().mockResolvedValue([]),
      persistMessage: vi.fn().mockResolvedValue(undefined),
      writeMemory: vi.fn().mockResolvedValue(undefined),
      canAutoWander: vi.fn().mockResolvedValue(true),
      updatePetState: vi.fn().mockResolvedValue(undefined),
      onMessage,
    });

    await runtime.handleUserMessage("你是谁？");

    expect(onMessage).toHaveBeenLastCalledWith(
      expect.objectContaining({ role: "assistant", content: "我是 Remi。" }),
      expect.objectContaining({ type: "USER_MESSAGE" }),
    );
    expect(traceAction).toHaveBeenCalledWith(
      expect.objectContaining({
        actionType: "response_parse_fallback",
        success: false,
      }),
    );
  });
});
