import { useCallback, useEffect, useRef, useState } from "react";
import { emitTo, listen } from "@tauri-apps/api/event";
import { PetPackPanel } from "../pet/PetPackPanel";
import { MemoryViewer } from "../memory/MemoryViewer";
import { updatePetState, type PetState } from "../pet/petState";
import type { ProviderCatalog } from "../providers/config";
import { ProvidersPanel } from "../providers/ProvidersPanel";
import { SoulEditor } from "../soul/SoulEditor";
import { ProfileControls } from "../profile/ProfileControls";
import {
  AGENT_HEARTBEAT_FINISHED,
  RUN_AGENT_HEARTBEAT,
} from "../windows/events";
import {
  getRuntimeOverview,
  updateAppSettings,
  type AppSettings,
  type RuntimeOverview,
} from "./settings";

type SettingsSection =
  | "General"
  | "Appearance"
  | "Behavior"
  | "Agent"
  | "Memory"
  | "Providers"
  | "Advanced";

const SECTIONS: SettingsSection[] = [
  "General",
  "Appearance",
  "Behavior",
  "Agent",
  "Memory",
  "Providers",
  "Advanced",
];

type SettingsPanelProps = {
  settings: AppSettings;
  petState: PetState;
  catalog: ProviderCatalog;
  onSettingsChange: (settings: AppSettings) => void;
  onPetStateChange: (state: PetState) => void;
  onCatalogChange: (catalog: ProviderCatalog) => void;
  onError: (message?: string) => void;
  previewMode?: boolean;
};

export function SettingsPanel({
  settings,
  petState,
  catalog,
  onSettingsChange,
  onPetStateChange,
  onCatalogChange,
  onError,
  previewMode = false,
}: SettingsPanelProps) {
  const [section, setSection] = useState<SettingsSection>("General");
  const [draft, setDraft] = useState(settings);
  const [overview, setOverview] = useState<RuntimeOverview>();
  const [heartbeatRunning, setHeartbeatRunning] = useState(false);
  const [soulKey, setSoulKey] = useState(0);
  const contentRef = useRef<HTMLDivElement>(null);

  const refreshOverview = useCallback(() => {
    if (previewMode) return;
    void getRuntimeOverview()
      .then(setOverview)
      .catch((error: unknown) => onError(String(error)));
  }, [onError, previewMode]);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);
  useEffect(() => {
    refreshOverview();
  }, [refreshOverview]);
  useEffect(() => {
    contentRef.current?.scrollTo({ top: 0 });
  }, [section]);
  useEffect(() => {
    if (previewMode) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen(AGENT_HEARTBEAT_FINISHED, () => {
      setHeartbeatRunning(false);
      refreshOverview();
    }).then((next) => {
      if (disposed) next();
      else unlisten = next;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [previewMode, refreshOverview]);

  async function saveSettings() {
    onError(undefined);
    try {
      const saved = await updateAppSettings(draft);
      setDraft(saved);
      onSettingsChange(saved);
      refreshOverview();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  async function setOpacity(opacity: number) {
    onError(undefined);
    try {
      onPetStateChange(await updatePetState({ opacity }));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <div className="settings-layout">
      <aside className="settings-nav" aria-label="Settings sections">
        {SECTIONS.map((name) => (
          <button
            type="button"
            className={section === name ? "active" : ""}
            key={name}
            onClick={() => setSection(name)}
          >
            {name}
          </button>
        ))}
      </aside>

      <div className="settings-content" ref={contentRef}>
        {section === "General" && (
          <section className="settings-section">
            <h2>General</h2>
            <label>
              Pet Name
              <input
                value={draft.petName}
                onChange={(event) =>
                  setDraft({ ...draft, petName: event.target.value })
                }
              />
            </label>
            <p className="settings-note">
              This name is part of the companion profile, not the selected LLM.
            </p>
            <button type="button" onClick={() => void saveSettings()}>
              Save General
            </button>
            <ProfileControls
              onError={onError}
              onImported={() => {
                setSoulKey((key) => key + 1);
                refreshOverview();
              }}
            />
          </section>
        )}

        {section === "Appearance" && (
          <section className="settings-section">
            <h2>Appearance</h2>
            <PetPackPanel
              petName={settings.petName}
              opacity={petState.opacity}
              onError={onError}
              previewMode={previewMode}
            />
            <label>
              Pet Size
              <select
                value={draft.petSize}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    petSize: event.target.value as AppSettings["petSize"],
                  })
                }
              >
                <option value="small">Small (128)</option>
                <option value="medium">Medium (160)</option>
                <option value="large">Large (200)</option>
              </select>
            </label>
            <label>
              Pet Opacity: {Math.round(petState.opacity * 100)}%
              <input
                type="range"
                min="20"
                max="100"
                value={Math.round(petState.opacity * 100)}
                onChange={(event) =>
                  void setOpacity(Number(event.target.value) / 100)
                }
              />
            </label>
            <button type="button" onClick={() => void saveSettings()}>
              Save Appearance
            </button>
          </section>
        )}

        {section === "Behavior" && (
          <section className="settings-section">
            <h2>Behavior</h2>
            <Toggle
              label="Auto Wander"
              checked={draft.autoWander}
              onChange={(autoWander) => setDraft({ ...draft, autoWander })}
            />
            <label>
              Wander Interval: {draft.wanderIntervalSeconds}s
              <input
                type="range"
                min="20"
                max="120"
                step="5"
                disabled={!draft.autoWander}
                value={draft.wanderIntervalSeconds}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    wanderIntervalSeconds: Number(event.target.value),
                  })
                }
              />
            </label>
            <label>
              Movement Speed
              <select
                disabled={!draft.autoWander}
                value={draft.movementSpeed}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    movementSpeed: event.target
                      .value as AppSettings["movementSpeed"],
                  })
                }
              >
                <option value="slow">Slow</option>
                <option value="normal">Normal</option>
                <option value="fast">Fast</option>
              </select>
            </label>
            <Toggle
              label="Proactive Interaction"
              checked={draft.proactiveInteraction}
              onChange={(proactiveInteraction) =>
                setDraft({ ...draft, proactiveInteraction })
              }
            />
            <label>
              Proactive Frequency
              <select
                disabled={!draft.proactiveInteraction}
                value={draft.proactiveFrequency}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    proactiveFrequency: event.target
                      .value as AppSettings["proactiveFrequency"],
                  })
                }
              >
                <option value="low">Low</option>
                <option value="normal">Normal</option>
                <option value="high">High</option>
              </select>
            </label>
            <Toggle
              label="Autonomous Heartbeat"
              checked={draft.agentHeartbeat}
              onChange={(agentHeartbeat) =>
                setDraft({ ...draft, agentHeartbeat })
              }
            />
            <Toggle
              label="Do Not Disturb"
              checked={draft.doNotDisturb}
              onChange={(doNotDisturb) => setDraft({ ...draft, doNotDisturb })}
            />
            <Toggle
              label="Quiet Hours"
              checked={draft.quietHoursEnabled}
              onChange={(quietHoursEnabled) =>
                setDraft({ ...draft, quietHoursEnabled })
              }
            />
            {draft.quietHoursEnabled && (
              <div className="quiet-hours-grid">
                <label>
                  From
                  <input
                    type="time"
                    value={draft.quietHoursStart}
                    onChange={(event) =>
                      setDraft({
                        ...draft,
                        quietHoursStart: event.target.value,
                      })
                    }
                  />
                </label>
                <label>
                  Until
                  <input
                    type="time"
                    value={draft.quietHoursEnd}
                    onChange={(event) =>
                      setDraft({ ...draft, quietHoursEnd: event.target.value })
                    }
                  />
                </label>
              </div>
            )}
            <label>
              Agent Heartbeat Interval: {draft.agentHeartbeatIntervalSeconds}s
              <input
                type="range"
                min="30"
                max="600"
                step="30"
                disabled={!draft.agentHeartbeat}
                value={draft.agentHeartbeatIntervalSeconds}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    agentHeartbeatIntervalSeconds: Number(event.target.value),
                  })
                }
              />
            </label>
            <label>
              Proactive Cooldown: {draft.proactiveCooldownMinutes} min
              <input
                type="range"
                min="5"
                max="120"
                step="5"
                disabled={!draft.proactiveInteraction}
                value={draft.proactiveCooldownMinutes}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    proactiveCooldownMinutes: Number(event.target.value),
                  })
                }
              />
            </label>
            <label>
              Maximum Proactive Messages / Hour
              <select
                disabled={!draft.proactiveInteraction}
                value={draft.maxProactiveMessagesPerHour}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    maxProactiveMessagesPerHour: Number(event.target.value),
                  })
                }
              >
                <option value="0">Never</option>
                <option value="1">1</option>
                <option value="2">2</option>
                <option value="3">3</option>
                <option value="5">5</option>
              </select>
            </label>
            <p className="settings-note">
              Body motion stays local. Agent Heartbeat may choose noop, wander,
              sleep, or speak; proactive speech is blocked unless all controls
              above allow it.
            </p>
            <div className="runtime-card">
              <strong>Last proactive action</strong>
              {overview?.lastProactiveAction ? (
                <>
                  <span>{overview.lastProactiveAction.actionType}</span>
                  <time>
                    {new Date(
                      overview.lastProactiveAction.timestamp,
                    ).toLocaleString()}
                  </time>
                  <small>
                    {shortActionReason(overview.lastProactiveAction)}
                  </small>
                </>
              ) : (
                <span>No heartbeat action recorded yet.</span>
              )}
            </div>
            <button type="button" onClick={() => void saveSettings()}>
              Save Behavior
            </button>
          </section>
        )}

        {section === "Agent" && (
          <section className="settings-section agent-settings">
            <div className="section-heading">
              <div>
                <h2>Agent</h2>
                <p>
                  SOUL and current Pet State remain independent from the active
                  model.
                </p>
              </div>
              <button
                type="button"
                onClick={() => {
                  setSoulKey((key) => key + 1);
                  refreshOverview();
                }}
              >
                Reload Soul
              </button>
            </div>
            <pre className="state-summary">
              {JSON.stringify(overview?.petState ?? petState, null, 2)}
            </pre>
            <SoulEditor key={soulKey} onError={onError} />
          </section>
        )}

        {section === "Memory" && (
          <MemoryViewer
            onError={onError}
            onChanged={refreshOverview}
            previewMode={previewMode}
          />
        )}

        {section === "Providers" && (
          <ProvidersPanel
            catalog={catalog}
            onChange={onCatalogChange}
            onError={onError}
          />
        )}

        {section === "Advanced" && (
          <section className="settings-section">
            <h2>Advanced</h2>
            <button
              type="button"
              disabled={heartbeatRunning}
              onClick={() => {
                onError(undefined);
                setHeartbeatRunning(true);
                void emitTo("pet-window", RUN_AGENT_HEARTBEAT).catch(
                  (error: unknown) => {
                    setHeartbeatRunning(false);
                    onError(String(error));
                  },
                );
              }}
            >
              {heartbeatRunning ? "Heartbeat Running…" : "Run Heartbeat Once"}
            </button>
            <p>
              Trace / Logs: {overview?.traceCount ?? "…"} recorded event/action
              rows
            </p>
            <p className="settings-note">
              This button runs the real Agent Runtime once. It may call the
              active model; it is not a synthetic preview.
            </p>
          </section>
        )}
      </div>
    </div>
  );
}

function shortActionReason(
  action: NonNullable<RuntimeOverview["lastProactiveAction"]>,
) {
  if (action.actionType === "speak_blocked" && action.reason) {
    return action.reason.replaceAll("_", " ");
  }
  if (!action.success) return "runtime or provider failure";
  if (!action.payloadJson) return action.success ? "completed" : "blocked";
  try {
    const payload = JSON.parse(action.payloadJson) as {
      reason?: string;
      text?: string;
    };
    return (payload.reason ?? payload.text ?? "completed").slice(0, 100);
  } catch {
    return action.success ? "completed" : "blocked";
  }
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="settings-toggle">
      <span>{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}
