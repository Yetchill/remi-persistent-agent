import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChatPanel } from "./chat/ChatPanel";
import { PetWindow } from "./windows/PetWindow";
import { SettingsWindow } from "./windows/SettingsWindow";

export default function App() {
  const preview = import.meta.env.DEV
    ? new URLSearchParams(window.location.search).get("preview")
    : undefined;
  const label = preview ?? getCurrentWindow().label;
  if (label === "settings-window")
    return <SettingsWindow preview={Boolean(preview)} />;
  if (label === "chat-bubble-window")
    return <ChatPanel preview={Boolean(preview)} />;
  return <PetWindow />;
}
