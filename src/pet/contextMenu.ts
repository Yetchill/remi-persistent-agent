import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type PetContextMenuActions = {
  autoWander: boolean;
  sleeping: boolean;
  onChat: () => void;
  onSettings: () => void;
  onToggleAutoWander: () => void;
  onToggleSleep: () => void;
  onClearConversation: () => void;
  onQuit: () => void;
};

export async function showPetContextMenu(actions: PetContextMenuActions) {
  const menu = await Menu.new({
    items: [
      { id: "chat", text: "Chat", action: actions.onChat },
      { id: "settings", text: "Settings", action: actions.onSettings },
      { item: "Separator" },
      {
        id: "auto-wander",
        text: "Auto Wander",
        checked: actions.autoWander,
        action: actions.onToggleAutoWander,
      },
      {
        id: "sleep-wake",
        text: actions.sleeping ? "Wake" : "Sleep",
        action: actions.onToggleSleep,
      },
      { item: "Separator" },
      {
        id: "clear-conversation",
        text: "Clear Current Conversation…",
        action: actions.onClearConversation,
      },
      { item: "Separator" },
      { id: "quit", text: "Quit Remi", action: actions.onQuit },
    ],
  });
  try {
    await menu.popup(undefined, getCurrentWindow());
  } finally {
    await menu.close();
  }
}
