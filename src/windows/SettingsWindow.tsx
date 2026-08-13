import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { ModelSelector } from "../providers/ModelSelector";
import {
  getProviderCatalog,
  PROVIDER_CATALOG_CHANGED,
  type ProviderCatalog,
} from "../providers/config";
import type { PetState } from "../pet/petState";
import { SettingsPanel } from "../settings/SettingsPanel";
import {
  DEFAULT_SETTINGS,
  getAppSettings,
  type AppSettings,
} from "../settings/settings";
import {
  PET_STATE_CHANGED,
  PROFILE_IMPORTED,
  SETTINGS_CHANGED,
} from "./events";

const EMPTY_PET_STATE: PetState = {
  energy: 100,
  boredom: 0,
  mood: "neutral",
  activity: "idle",
  x: 0,
  y: 0,
  opacity: 1,
};

export function SettingsWindow({ preview = false }: { preview?: boolean }) {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [petState, setPetState] = useState<PetState>(EMPTY_PET_STATE);
  const [catalog, setCatalog] = useState<ProviderCatalog>({ providers: [] });
  const [error, setError] = useState<string>();

  useEffect(() => {
    if (preview) return;
    const refresh = () =>
      void Promise.all([
        getAppSettings(),
        invoke<PetState>("get_pet_state"),
        getProviderCatalog(),
      ]).then(
        ([savedSettings, savedPetState, savedCatalog]) => {
          setSettings(savedSettings);
          setPetState(savedPetState);
          setCatalog(savedCatalog);
        },
        (caught: unknown) => setError(String(caught)),
      );
    refresh();
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const track = (promise: Promise<() => void>) =>
      void promise.then((unlisten) =>
        disposed ? unlisten() : unlisteners.push(unlisten),
      );
    track(
      listen<ProviderCatalog>(PROVIDER_CATALOG_CHANGED, (event) => {
        setCatalog(event.payload);
      }),
    );
    track(
      listen<AppSettings>(SETTINGS_CHANGED, (event) => {
        setSettings(event.payload);
      }),
    );
    track(listen(PROFILE_IMPORTED, refresh));
    track(
      getCurrentWindow().onCloseRequested((event) => {
        event.preventDefault();
        void invoke("hide_current_window");
      }),
    );
    track(
      getCurrentWindow().onFocusChanged(({ payload }) => {
        if (payload) refresh();
      }),
    );
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [preview]);

  function applySettings(next: AppSettings) {
    setSettings(next);
    void emitTo("pet-window", SETTINGS_CHANGED, next).catch((caught) =>
      setError(String(caught)),
    );
    void emitTo("chat-bubble-window", SETTINGS_CHANGED, next).catch((caught) =>
      setError(String(caught)),
    );
  }

  function applyPetState(next: PetState) {
    setPetState(next);
    void emitTo("pet-window", PET_STATE_CHANGED, next).catch((caught) =>
      setError(String(caught)),
    );
  }

  return (
    <main className="settings-window-shell">
      <header className="settings-window-header" data-tauri-drag-region>
        <strong data-tauri-drag-region>Remi Settings</strong>
        <div className="settings-header-actions">
          <ModelSelector
            compact
            catalog={catalog}
            onChange={setCatalog}
            onError={setError}
          />
          <button
            type="button"
            aria-label="Close Settings"
            onClick={() => void invoke("hide_current_window")}
          >
            ×
          </button>
        </div>
      </header>
      <SettingsPanel
        settings={settings}
        petState={petState}
        catalog={catalog}
        onSettingsChange={applySettings}
        onPetStateChange={applyPetState}
        onCatalogChange={setCatalog}
        onError={preview ? () => undefined : setError}
        previewMode={preview}
      />
      {error && <p className="settings-window-error">{error}</p>}
    </main>
  );
}
