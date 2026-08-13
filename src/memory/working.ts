import { invoke } from "@tauri-apps/api/core";
import type { ConversationMessage } from "../agent/context";

export function persistWorkingMessage(message: ConversationMessage) {
  return invoke<void>("persist_message", { message });
}

export function getRecentWorkingMessages(limit = 50) {
  return invoke<ConversationMessage[]>("get_recent_messages", { limit });
}

export function clearCurrentConversation() {
  return invoke<number>("clear_current_conversation");
}
