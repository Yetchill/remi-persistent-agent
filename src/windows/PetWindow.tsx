import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState } from "react";
import type { AgentAction } from "../agent/actions";
import type { ConversationMessage } from "../agent/context";
import type { AgentEvent } from "../agent/events";
import { nextAgentHeartbeatDelayMs } from "../agent/heartbeat";
import { AgentRuntime } from "../agent/runtime";
import {
  estimateSpeechBubbleDuration,
  type BubblePlacement,
} from "../chat/bubbleState";
import {
  estimateTalkDuration,
  resolveVisualState,
  type PetVisualState,
  visualStateForActivity,
} from "../pet/animation";
import { runWanderLoop } from "../pet/motion";
import { showPetContextMenu } from "../pet/contextMenu";
import {
  listPetPacks,
  onPetPackChanged,
  releasePetPack,
  resolvePetPack,
  type PetPack,
  type ResolvedPetPack,
} from "../pet/packs";
import {
  recoverActivity,
  updatePetState,
  type PetState,
} from "../pet/petState";
import { PetView } from "../pet/PetView";
import type { PetPosition, WorkArea } from "../pet/screen";
import { clearCurrentConversation } from "../memory/working";
import {
  DEFAULT_SETTINGS,
  getAppSettings,
  movementDuration,
  type AppSettings,
  updateAppSettings,
} from "../settings/settings";
import {
  AGENT_HEARTBEAT_FINISHED,
  BUBBLE_AGENT_MESSAGE,
  BUBBLE_CONVERSATION_CLEARED,
  BUBBLE_PLACEMENT_CHANGED,
  BUBBLE_REQUEST_FINISHED,
  BUBBLE_USER_MESSAGE,
  PET_STATE_CHANGED,
  PROFILE_IMPORTED,
  RUN_AGENT_HEARTBEAT,
  SETTINGS_CHANGED,
  type BubbleRequestResult,
} from "./events";

const DRAG_WANDER_COOLDOWN_MS = 15_000;
const DEFAULT_PET_STATE: PetState = {
  energy: 100,
  boredom: 0,
  mood: "neutral",
  activity: "idle",
  x: 0,
  y: 0,
  opacity: 1,
};

export function PetWindow() {
  const [ready, setReady] = useState(false);
  const [initialized, setInitialized] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [wanderCooldownUntil, setWanderCooldownUntil] = useState(0);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [petState, setPetState] = useState<PetState>(DEFAULT_PET_STATE);
  const [visualState, setVisualState] = useState<PetVisualState>("idle");
  const [petPack, setPetPack] = useState<ResolvedPetPack>();
  const runtimeRef = useRef<AgentRuntime | null>(null);
  const settingsRef = useRef<AppSettings>(DEFAULT_SETTINGS);
  const petStateRef = useRef<PetState>(DEFAULT_PET_STATE);
  const visualStateRef = useRef<PetVisualState>("idle");
  const visualTimerRef = useRef<number | undefined>(undefined);
  const talkFinishedRef = useRef<Promise<void> | undefined>(undefined);
  const resolveTalkRef = useRef<(() => void) | undefined>(undefined);
  const petPackRef = useRef<ResolvedPetPack | undefined>(undefined);
  const petPackRequestRef = useRef(0);
  const petPackMountedRef = useRef(true);
  const heartbeatInFlightRef = useRef<Promise<void> | null>(null);

  const applyPetPack = useCallback(async (pack: PetPack) => {
    const request = ++petPackRequestRef.current;
    const resolved = await resolvePetPack(pack);
    if (!petPackMountedRef.current || request !== petPackRequestRef.current) {
      releasePetPack(resolved);
      return;
    }
    const previous = petPackRef.current;
    petPackRef.current = resolved;
    setPetPack(resolved);
    if (previous) releasePetPack(previous);
  }, []);

  const applyPetState = useCallback((next: PetState) => {
    petStateRef.current = next;
    setPetState(next);
  }, []);

  const runAgentHeartbeat = useCallback(() => {
    if (heartbeatInFlightRef.current) return heartbeatInFlightRef.current;
    const run = (async () => {
      try {
        await runtimeRef.current?.recordAgentHeartbeat();
      } finally {
        heartbeatInFlightRef.current = null;
      }
    })();
    heartbeatInFlightRef.current = run;
    return run;
  }, []);

  const applyVisualState = useCallback((next: PetVisualState) => {
    visualStateRef.current = next;
    setVisualState(next);
  }, []);

  const stopTimedVisual = useCallback(() => {
    if (visualTimerRef.current !== undefined) {
      window.clearTimeout(visualTimerRef.current);
      visualTimerRef.current = undefined;
    }
    resolveTalkRef.current?.();
    resolveTalkRef.current = undefined;
  }, []);

  const startTalkVisual = useCallback(
    (text: string) => {
      stopTimedVisual();
      applyVisualState("talk");
      talkFinishedRef.current = new Promise<void>((resolve) => {
        resolveTalkRef.current = resolve;
      });
      visualTimerRef.current = window.setTimeout(() => {
        visualTimerRef.current = undefined;
        if (visualStateRef.current === "talk") {
          applyVisualState("idle");
          void updatePetState({ activity: "idle" }).then(applyPetState);
        }
        resolveTalkRef.current?.();
        resolveTalkRef.current = undefined;
      }, estimateTalkDuration(text));
    },
    [applyPetState, applyVisualState, stopTimedVisual],
  );

  const receiveMessage = useCallback((message: ConversationMessage) => {
    void emitTo("chat-bubble-window", BUBBLE_AGENT_MESSAGE, message);
  }, []);

  const receiveExecutedAction = useCallback(
    async (event: AgentEvent, action: AgentAction) => {
      if (action.type === "speak") {
        if (visualStateRef.current !== "sleep") startTalkVisual(action.text);
        if (event.type === "AGENT_HEARTBEAT" || event.type === "USER_MESSAGE") {
          await invoke<BubblePlacement | null>("open_speech_bubble", {
            text: action.text,
            durationMs: estimateSpeechBubbleDuration(action.text),
            source:
              event.type === "AGENT_HEARTBEAT"
                ? "proactive"
                : "user_conversation",
          });
        }
        return;
      }
      if (action.type === "sleep") {
        stopTimedVisual();
        applyVisualState("sleep");
        applyPetState({
          ...petStateRef.current,
          activity: "sleeping",
          mood: "sleepy",
        });
        return;
      }
      if (action.type === "wake") {
        stopTimedVisual();
        applyVisualState("idle");
        applyPetState({
          ...petStateRef.current,
          activity: "idle",
          mood: "neutral",
        });
        return;
      }
      if (action.type === "set_activity") {
        const resolved = resolveVisualState([
          visualStateRef.current,
          visualStateForActivity(action.activity),
        ]);
        if (resolved === visualStateRef.current) return;
        stopTimedVisual();
        applyVisualState(resolved);
        applyPetState({ ...petStateRef.current, activity: action.activity });
      }
    },
    [applyPetState, applyVisualState, startTalkVisual, stopTimedVisual],
  );

  if (!runtimeRef.current) {
    runtimeRef.current = AgentRuntime.createDesktopRuntime(
      receiveMessage,
      receiveExecutedAction,
    );
  }

  useEffect(() => {
    void Promise.all([
      getAppSettings(),
      invoke<PetState>("get_pet_state"),
      listPetPacks(),
    ])
      .then(([savedSettings, savedState, catalog]) => {
        setSettings(savedSettings);
        settingsRef.current = savedSettings;
        applyPetState(savedState);
        applyVisualState(visualStateForActivity(savedState.activity));
        const activePack = catalog.packs.find(
          (pack) => pack.id === catalog.activePetPackId,
        );
        if (activePack) void applyPetPack(activePack);
        setInitialized(true);
        return runtimeRef.current?.recordAppStarted();
      })
      .catch((error: unknown) => {
        console.error("Failed to initialize Remi", error);
        setReady(true);
      });

    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const track = (promise: Promise<() => void>) =>
      void promise.then((unlisten) =>
        disposed ? unlisten() : unlisteners.push(unlisten),
      );
    track(
      listen<string>(BUBBLE_USER_MESSAGE, async (event) => {
        let result: BubbleRequestResult = { ok: true };
        stopTimedVisual();
        talkFinishedRef.current = undefined;
        applyVisualState("think");
        try {
          if (petStateRef.current.activity === "sleeping") {
            applyPetState(
              await updatePetState({ activity: "idle", mood: "neutral" }),
            );
          }
          await runtimeRef.current?.handleUserMessage(event.payload);
          await talkFinishedRef.current;
        } catch (error) {
          result = {
            ok: false,
            error: "唔……刚刚没连上。请稍后再试。",
          };
          console.error("Agent response failed", error);
        }
        if (visualStateRef.current !== "sleep") applyVisualState("idle");
        await emitTo("chat-bubble-window", BUBBLE_REQUEST_FINISHED, result);
      }),
    );
    track(
      listen<AppSettings>(SETTINGS_CHANGED, (event) => {
        settingsRef.current = event.payload;
        setSettings(event.payload);
      }),
    );
    track(
      listen<PetState>(PET_STATE_CHANGED, (event) => {
        stopTimedVisual();
        applyPetState(event.payload);
        applyVisualState(visualStateForActivity(event.payload.activity));
      }),
    );
    track(
      listen(RUN_AGENT_HEARTBEAT, async () => {
        try {
          await runAgentHeartbeat();
        } catch (error) {
          console.error("Manual Agent heartbeat failed", error);
        } finally {
          await emitTo("settings-window", AGENT_HEARTBEAT_FINISHED);
        }
      }),
    );
    track(
      onPetPackChanged((pack) => {
        void applyPetPack(pack).catch((error: unknown) =>
          console.error("Failed to switch Pet Pack", error),
        );
      }),
    );
    track(
      listen(PROFILE_IMPORTED, async () => {
        try {
          const [savedSettings, catalog] = await Promise.all([
            getAppSettings(),
            listPetPacks(),
          ]);
          settingsRef.current = savedSettings;
          setSettings(savedSettings);
          await emitTo("chat-bubble-window", SETTINGS_CHANGED, savedSettings);
          const activePack = catalog.packs.find(
            (pack) => pack.id === catalog.activePetPackId,
          );
          if (activePack) await applyPetPack(activePack);
        } catch (error) {
          console.error("Failed to apply imported profile", error);
        }
      }),
    );
    track(
      getCurrentWindow().onMoved(() => {
        void syncBubblePlacement();
      }),
    );
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [
    applyPetPack,
    applyPetState,
    applyVisualState,
    runAgentHeartbeat,
    stopTimedVisual,
  ]);

  useEffect(() => {
    petPackMountedRef.current = true;
    return () => {
      petPackMountedRef.current = false;
      petPackRequestRef.current += 1;
      if (petPackRef.current) releasePetPack(petPackRef.current);
      petPackRef.current = undefined;
    };
  }, []);

  useEffect(() => {
    const onClosing = () => {
      void runtimeRef.current?.recordAppClosing();
    };
    window.addEventListener("beforeunload", onClosing);
    return () => window.removeEventListener("beforeunload", onClosing);
  }, []);

  useEffect(
    () => () => {
      stopTimedVisual();
    },
    [stopTimedVisual],
  );

  useEffect(() => {
    if (wanderCooldownUntil <= Date.now()) return;
    const timeout = window.setTimeout(
      () => setWanderCooldownUntil(0),
      wanderCooldownUntil - Date.now(),
    );
    return () => window.clearTimeout(timeout);
  }, [wanderCooldownUntil]);

  useEffect(() => {
    if (!initialized) return;
    const bodyTimer = window.setInterval(() => {
      void runtimeRef.current
        ?.recordBodyHeartbeat()
        .catch((error: unknown) =>
          console.error("Body heartbeat trace failed", error),
        );
    }, 30_000);
    return () => window.clearInterval(bodyTimer);
  }, [initialized]);

  useEffect(() => {
    if (!initialized || !settings.agentHeartbeat) return;
    let cancelled = false;
    let agentTimer: number | undefined;
    const scheduleNext = () => {
      agentTimer = window.setTimeout(() => {
        void runAgentHeartbeat()
          .catch((error: unknown) =>
            console.error("Agent heartbeat failed", error),
          )
          .finally(() => {
            if (!cancelled) scheduleNext();
          });
      }, nextAgentHeartbeatDelayMs(settings.agentHeartbeatIntervalSeconds));
    };
    scheduleNext();
    return () => {
      cancelled = true;
      if (agentTimer !== undefined) window.clearTimeout(agentTimer);
    };
  }, [
    initialized,
    runAgentHeartbeat,
    settings.agentHeartbeat,
    settings.agentHeartbeatIntervalSeconds,
  ]);

  useEffect(() => {
    const sleeping = visualState === "sleep";
    if (
      !initialized ||
      dragging ||
      sleeping ||
      wanderCooldownUntil > Date.now()
    ) {
      return;
    }
    const abortController = new AbortController();
    async function startMotion() {
      await invoke("set_pet_window_size", { petSizeName: settings.petSize });
      const workArea = await invoke<WorkArea>("get_work_area");
      const position = await invoke<PetPosition>("set_pet_position", {
        x: petState.x,
        y: petState.y,
      });
      const activity = recoverActivity(petState.activity);
      const restored = await updatePetState({ ...position, activity });
      applyPetState(restored);
      setReady(true);
      if (!settings.autoWander || activity === "sleeping") return;
      await runWanderLoop({
        workArea,
        initialPosition: position,
        move: (target) => invoke("set_pet_position", target),
        onMoveStart: async () =>
          applyPetState(await updatePetState({ activity: "wandering" })),
        onMoveFinished: async (target) => {
          applyPetState(await updatePetState({ ...target, activity: "idle" }));
          await syncBubblePlacement();
        },
        wanderDelayMs: settings.wanderIntervalSeconds * 1_000,
        moveDurationMs: movementDuration(settings.movementSpeed),
        canMove: async () => (await getAppSettings()).autoWander,
        signal: abortController.signal,
      });
    }
    void startMotion().catch((error: unknown) => {
      console.error("Failed to start local motion engine", error);
    });
    return () => abortController.abort();
  }, [
    initialized,
    dragging,
    wanderCooldownUntil,
    settings.autoWander,
    settings.wanderIntervalSeconds,
    settings.movementSpeed,
    settings.petSize,
    visualState === "sleep",
    applyPetState,
  ]);

  async function emitBubblePlacement(placement: BubblePlacement) {
    await emitTo("chat-bubble-window", BUBBLE_PLACEMENT_CHANGED, placement);
  }

  async function syncBubblePlacement() {
    const placement = await invoke<BubblePlacement>(
      "sync_chat_bubble_position",
    );
    await emitBubblePlacement(placement);
  }

  async function wakeIfNeeded() {
    if (visualStateRef.current === "sleep") {
      stopTimedVisual();
      applyVisualState("idle");
      applyPetState(
        await updatePetState({ activity: "idle", mood: "neutral" }),
      );
    }
  }

  async function forceOpenInteractiveBubble() {
    await wakeIfNeeded();
    await invoke<BubblePlacement>("open_chat_bubble");
    await runtimeRef.current?.recordPetClick();
  }

  async function openBubble() {
    if (await invoke<boolean>("is_chat_bubble_visible")) return;
    await forceOpenInteractiveBubble();
  }

  async function toggleAutoWander() {
    const next = await updateAppSettings({
      ...settingsRef.current,
      autoWander: !settingsRef.current.autoWander,
    });
    settingsRef.current = next;
    setSettings(next);
    await Promise.all([
      emitTo("chat-bubble-window", SETTINGS_CHANGED, next),
      emitTo("settings-window", SETTINGS_CHANGED, next),
    ]);
  }

  async function toggleSleep() {
    const sleeping = visualStateRef.current === "sleep";
    stopTimedVisual();
    const next = await updatePetState(
      sleeping
        ? { activity: "idle", mood: "neutral" }
        : { activity: "sleeping", mood: "sleepy" },
    );
    applyPetState(next);
    applyVisualState(sleeping ? "idle" : "sleep");
  }

  function openContextMenu() {
    void showPetContextMenu({
      autoWander: settingsRef.current.autoWander,
      sleeping: visualStateRef.current === "sleep",
      onChat: () => void forceOpenInteractiveBubble(),
      onSettings: () => void invoke("open_settings_window"),
      onToggleAutoWander: () => void toggleAutoWander(),
      onToggleSleep: () => void toggleSleep(),
      onClearConversation: () => {
        if (
          window.confirm(
            "Clear only the recent conversation? Long-term memories will be kept.",
          )
        ) {
          void clearCurrentConversation()
            .then(async () => {
              await runtimeRef.current?.clearRecentConversation();
              await emitTo("chat-bubble-window", BUBBLE_CONVERSATION_CLEARED);
            })
            .catch((error: unknown) =>
              console.error("Failed to clear current conversation", error),
            );
        }
      },
      onQuit: () => {
        void runtimeRef.current
          ?.recordAppClosing()
          .finally(() => void invoke("quit_app"));
      },
    }).catch((error: unknown) =>
      console.error("Failed to open Pet context menu", error),
    );
  }

  async function finishDrag(position: PetPosition) {
    try {
      const legalPosition = await invoke<PetPosition>(
        "set_pet_position",
        position,
      );
      const next = await updatePetState(legalPosition);
      applyPetState(next);
      await runtimeRef.current?.recordPetDrag(legalPosition);
      await syncBubblePlacement();
      setWanderCooldownUntil(Date.now() + DRAG_WANDER_COOLDOWN_MS);
    } finally {
      setDragging(false);
    }
  }

  return (
    <main
      className="pet-window-shell"
      data-ready={ready}
      data-dragging={dragging}
    >
      <PetView
        name={settings.petName}
        opacity={petState.opacity}
        visualState={visualState}
        pack={petPack}
        onClick={() => void openBubble()}
        onContextMenu={openContextMenu}
        onDragStart={() => setDragging(true)}
        onDragEnd={(position) => void finishDrag(position)}
        onDragCancel={() => setDragging(false)}
      />
    </main>
  );
}
