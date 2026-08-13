import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { type FormEvent, useEffect, useRef, useState } from "react";
import type { ConversationMessage } from "../agent/context";
import type { BubblePlacement, BubbleState } from "./bubbleState";
import { getRecentWorkingMessages } from "../memory/working";
import {
  getProviderCatalog,
  PROVIDER_CATALOG_CHANGED,
  type ProviderCatalog,
} from "../providers/config";
import { getAppSettings, type AppSettings } from "../settings/settings";
import {
  BUBBLE_AGENT_MESSAGE,
  BUBBLE_CONVERSATION_CLEARED,
  BUBBLE_OPEN_INTERACTIVE,
  BUBBLE_PLACEMENT_CHANGED,
  BUBBLE_REQUEST_FINISHED,
  BUBBLE_SHOW_PROACTIVE,
  BUBBLE_USER_MESSAGE,
  SETTINGS_CHANGED,
  type BubbleRequestResult,
  type ProactiveBubbleEvent,
} from "../windows/events";

const INITIAL_BUBBLE_STATE: BubbleState = {
  mode: "hidden",
  text: "",
  placement: "above",
};

const PREVIEW_BUBBLE_STATE: BubbleState = {
  mode: "interactive",
  text: "今天也想和你待在一起。",
  source: "user_conversation",
  placement: "above",
};

export function ChatPanel({ preview = false }: { preview?: boolean }) {
  const previewSpeech =
    preview &&
    new URLSearchParams(window.location.search).get("mode") === "speech";
  const initialBubble: BubbleState = preview
    ? {
        ...PREVIEW_BUBBLE_STATE,
        mode: previewSpeech ? "speech" : "interactive",
      }
    : INITIAL_BUBBLE_STATE;
  const [bubble, setBubble] = useState<BubbleState>(initialBubble);
  const [input, setInput] = useState("");
  const [requestPending, setRequestPending] = useState(false);
  const [petName, setPetName] = useState("Remi");
  const [catalog, setCatalog] = useState<ProviderCatalog>({ providers: [] });
  const inputRef = useRef<HTMLInputElement>(null);
  const bubbleRef = useRef<BubbleState>(initialBubble);
  const proactiveTimerRef = useRef<number | undefined>(undefined);

  function updateBubble(
    update: Partial<BubbleState> | ((current: BubbleState) => BubbleState),
  ) {
    setBubble((current) => {
      const next =
        typeof update === "function"
          ? update(current)
          : { ...current, ...update };
      bubbleRef.current = next;
      return next;
    });
  }

  function cancelProactiveTimer() {
    if (proactiveTimerRef.current !== undefined) {
      window.clearTimeout(proactiveTimerRef.current);
      proactiveTimerRef.current = undefined;
    }
  }

  function hideBubble() {
    cancelProactiveTimer();
    updateBubble({ mode: "hidden" });
    void invoke("hide_current_window");
  }

  async function openInteractive() {
    cancelProactiveTimer();
    updateBubble({ mode: "interactive" });
    try {
      const placement = await invoke<BubblePlacement>("open_chat_bubble");
      updateBubble({ placement });
      requestAnimationFrame(() => inputRef.current?.focus());
    } catch (caught) {
      console.error("Failed to expand chat bubble", caught);
    }
  }

  useEffect(() => {
    if (preview) return;
    void Promise.all([
      getProviderCatalog(),
      getRecentWorkingMessages(12),
      getAppSettings(),
    ]).then(
      ([nextCatalog, messages, savedSettings]) => {
        setCatalog(nextCatalog);
        setPetName(savedSettings.petName);
        const latest = messages
          .filter((message) => message.role === "assistant")
          .at(-1)?.content;
        if (latest) {
          updateBubble({ text: latest, source: "user_conversation" });
        }
      },
      (caught: unknown) => {
        console.error("Failed to prepare chat bubble", caught);
      },
    );
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const track = (promise: Promise<() => void>) =>
      void promise.then((unlisten) =>
        disposed ? unlisten() : unlisteners.push(unlisten),
      );
    track(
      listen<ConversationMessage>(BUBBLE_AGENT_MESSAGE, (event) => {
        if (event.payload.role === "assistant") {
          updateBubble({
            text: event.payload.content,
            source: "user_conversation",
          });
        }
      }),
    );
    track(
      listen(BUBBLE_CONVERSATION_CLEARED, () => {
        updateBubble({ text: "", source: undefined });
      }),
    );
    track(
      listen(BUBBLE_OPEN_INTERACTIVE, () => {
        cancelProactiveTimer();
        updateBubble({ mode: "interactive" });
        requestAnimationFrame(() => inputRef.current?.focus());
      }),
    );
    track(
      listen<ProactiveBubbleEvent>(BUBBLE_SHOW_PROACTIVE, (event) => {
        cancelProactiveTimer();
        updateBubble({
          mode: "speech",
          text: event.payload.text,
          source: "proactive",
        });
        proactiveTimerRef.current = window.setTimeout(() => {
          proactiveTimerRef.current = undefined;
          hideBubble();
        }, event.payload.durationMs);
      }),
    );
    track(
      listen<BubblePlacement>(BUBBLE_PLACEMENT_CHANGED, (event) => {
        updateBubble({ placement: event.payload });
      }),
    );
    track(
      listen<BubbleRequestResult>(BUBBLE_REQUEST_FINISHED, (event) => {
        setRequestPending(false);
        if (!event.payload.ok && event.payload.error) {
          updateBubble({
            text: event.payload.error,
            source: "user_conversation",
          });
        }
        inputRef.current?.focus();
      }),
    );
    track(
      listen<ProviderCatalog>(PROVIDER_CATALOG_CHANGED, (event) => {
        setCatalog(event.payload);
      }),
    );
    track(
      listen<AppSettings>(SETTINGS_CHANGED, (event) => {
        setPetName(event.payload.petName);
      }),
    );
    track(
      getCurrentWindow().onFocusChanged(({ payload }) => {
        if (!payload && bubbleRef.current.mode === "interactive") hideBubble();
      }),
    );
    track(
      getCurrentWindow().onCloseRequested((event) => {
        event.preventDefault();
        hideBubble();
      }),
    );
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") hideBubble();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      disposed = true;
      cancelProactiveTimer();
      unlisteners.forEach((unlisten) => unlisten());
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [preview]);

  const hasActiveModel = Boolean(
    catalog.activeProviderId && catalog.activeModelId,
  );

  async function submit(event: FormEvent) {
    event.preventDefault();
    const text = input.trim();
    if (!text || requestPending || !hasActiveModel) return;
    setInput("");
    setRequestPending(true);
    try {
      await emitTo("pet-window", BUBBLE_USER_MESSAGE, text);
    } catch (caught) {
      setRequestPending(false);
      console.error("Failed to send chat message", caught);
      updateBubble({
        text: "唔……刚刚没连上。请稍后再试。",
        source: "user_conversation",
      });
    }
  }

  if (bubble.mode === "hidden") return null;

  if (bubble.mode === "speech") {
    return (
      <button
        type="button"
        className="speech-bubble"
        data-placement={bubble.placement}
        aria-label={`和 ${petName} 聊天`}
        onClick={() => void openInteractive()}
      >
        <span>{bubble.text}</span>
      </button>
    );
  }

  const displayText =
    bubble.text ||
    (!hasActiveModel
      ? `${petName} 还没有连接模型。右键 → Settings → Providers`
      : "");

  return (
    <section
      className="chat-bubble"
      data-placement={bubble.placement}
      aria-label={`${petName} compact chat`}
    >
      {displayText && (
        <p className="latest-reply" aria-live="polite">
          {displayText}
        </p>
      )}
      <form className="bubble-input" onSubmit={(event) => void submit(event)}>
        <input
          ref={inputRef}
          value={input}
          onChange={(event) => setInput(event.target.value)}
          placeholder={hasActiveModel ? "跟我说点什么…" : "请先配置模型"}
          disabled={!hasActiveModel || requestPending}
        />
        <button
          type="submit"
          aria-label="Send"
          disabled={!input.trim() || requestPending || !hasActiveModel}
        >
          ➤
        </button>
      </form>
    </section>
  );
}
