import { useEffect, useMemo, useRef, useState } from "react";
import {
  createProviderFromPreset,
  deleteProvider,
  saveProvider,
  setActiveModel,
  type ModelConfig,
  type ProviderCatalog,
  type ProviderConfig,
  type ProviderPreset,
} from "./config";

type ProvidersPanelProps = {
  catalog: ProviderCatalog;
  onChange: (catalog: ProviderCatalog) => void;
  onError: (message?: string) => void;
};

function newModel(): ModelConfig {
  return {
    id: crypto.randomUUID(),
    displayName: "New Model",
    modelId: "",
    enabled: true,
    persisted: false,
  };
}

export function reconcileProviderCatalogDraft(
  current: ProviderCatalog,
  incoming: ProviderCatalog,
  dirty: boolean,
): ProviderCatalog {
  if (!dirty) return incoming;
  return {
    ...current,
    activeProviderId: incoming.activeProviderId,
    activeModelId: incoming.activeModelId,
  };
}

export function ProvidersPanel({
  catalog: externalCatalog,
  onChange,
  onError,
}: ProvidersPanelProps) {
  const [catalog, setCatalog] = useState(externalCatalog);
  const [selectedId, setSelectedId] = useState<string>();
  const [preset, setPreset] = useState<ProviderPreset>("custom");
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const dirtyRef = useRef(false);

  useEffect(() => {
    setCatalog((current) =>
      reconcileProviderCatalogDraft(current, externalCatalog, dirtyRef.current),
    );
  }, [externalCatalog]);

  useEffect(() => {
    if (
      selectedId &&
      catalog.providers.some((item) => item.id === selectedId)
    ) {
      return;
    }
    setSelectedId(catalog.activeProviderId ?? catalog.providers.at(0)?.id);
  }, [catalog.activeProviderId, catalog.providers, selectedId]);

  const provider = useMemo(
    () => catalog.providers.find((item) => item.id === selectedId),
    [catalog.providers, selectedId],
  );

  function updateDraft(next: ProviderCatalog) {
    dirtyRef.current = true;
    setCatalog(next);
    onChange(next);
  }

  function acceptPersisted(next: ProviderCatalog) {
    dirtyRef.current = false;
    setCatalog(next);
    onChange(next);
  }

  function updateProvider(update: (current: ProviderConfig) => ProviderConfig) {
    if (!provider) return;
    updateDraft({
      ...catalog,
      providers: catalog.providers.map((item) =>
        item.id === provider.id ? { ...update(item), persisted: false } : item,
      ),
    });
  }

  function addProvider() {
    const next = createProviderFromPreset(preset);
    updateDraft({ ...catalog, providers: [...catalog.providers, next] });
    setSelectedId(next.id);
  }

  async function persist(current: ProviderConfig) {
    setSaving(true);
    onError(undefined);
    try {
      const next = await saveProvider(current, apiKeys[current.id] ?? "");
      acceptPersisted(next);
      setApiKeys((keys) => ({ ...keys, [current.id]: "" }));
      return next;
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
      return undefined;
    } finally {
      setSaving(false);
    }
  }

  async function activate(current: ProviderConfig, model: ModelConfig) {
    const saved = await persist(current);
    if (!saved) return;
    try {
      acceptPersisted(await setActiveModel(current.id, model.id));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  async function remove(providerId: string) {
    onError(undefined);
    const target = catalog.providers.find((item) => item.id === providerId);
    if (target?.persisted === false) {
      const providers = catalog.providers.filter(
        (item) => item.id !== providerId,
      );
      updateDraft({ ...catalog, providers });
      setSelectedId(catalog.activeProviderId ?? providers.at(0)?.id);
      return;
    }
    try {
      const next = await deleteProvider(providerId);
      acceptPersisted(next);
      setSelectedId(next.activeProviderId ?? next.providers.at(0)?.id);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <section className="providers-master-detail">
      <aside className="provider-master">
        <div>
          <h2>Providers</h2>
          <p>OpenAI-compatible backends</p>
        </div>
        <div className="provider-list">
          {catalog.providers.map((item) => (
            <div className="provider-list-row" key={item.id}>
              <button
                type="button"
                className={item.id === selectedId ? "selected" : ""}
                onClick={() => setSelectedId(item.id)}
              >
                <span>{item.displayName}</span>
                {catalog.activeProviderId === item.id && (
                  <span className="active-dot" title="Active Provider">
                    ●
                  </span>
                )}
              </button>
              <button
                type="button"
                className="provider-delete"
                aria-label={`Delete ${item.displayName}`}
                onClick={() => void remove(item.id)}
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <div className="provider-add">
          <select
            aria-label="Provider preset"
            value={preset}
            onChange={(event) =>
              setPreset(event.target.value as ProviderPreset)
            }
          >
            <option value="openai">OpenAI</option>
            <option value="deepseek">DeepSeek</option>
            <option value="qwen">Qwen</option>
            <option value="kimi">Kimi</option>
            <option value="custom">Custom</option>
          </select>
          <button type="button" onClick={addProvider}>
            + Provider
          </button>
        </div>
      </aside>

      <div className="provider-detail">
        {!provider ? (
          <p className="settings-empty">Select or add a Provider.</p>
        ) : (
          <>
            <div className="section-heading">
              <div>
                <h2>{provider.displayName}</h2>
                <p>Provider Settings</p>
              </div>
              <label className="inline-toggle">
                <input
                  type="checkbox"
                  checked={provider.enabled}
                  onChange={(event) =>
                    updateProvider((current) => ({
                      ...current,
                      enabled: event.target.checked,
                    }))
                  }
                />
                Enabled
              </label>
            </div>
            <label>
              Display Name
              <input
                value={provider.displayName}
                onChange={(event) =>
                  updateProvider((current) => ({
                    ...current,
                    displayName: event.target.value,
                  }))
                }
              />
            </label>
            <label>
              Base URL
              <input
                value={provider.baseUrl}
                onChange={(event) =>
                  updateProvider((current) => ({
                    ...current,
                    baseUrl: event.target.value,
                  }))
                }
              />
            </label>
            <label>
              API Key
              <input
                type="password"
                autoComplete="off"
                value={apiKeys[provider.id] ?? ""}
                placeholder={
                  provider.hasApiKey
                    ? "•••••••••••••••• (loaded for this Provider)"
                    : "Optional for local APIs"
                }
                onChange={(event) =>
                  setApiKeys((keys) => ({
                    ...keys,
                    [provider.id]: event.target.value,
                  }))
                }
              />
            </label>

            <div className="models-heading">
              <strong>Models</strong>
              <button
                type="button"
                onClick={() =>
                  updateProvider((current) => ({
                    ...current,
                    models: [...current.models, newModel()],
                  }))
                }
              >
                + Add Model
              </button>
            </div>
            <div className="model-detail-list">
              {provider.models.map((model) => {
                const active =
                  catalog.activeProviderId === provider.id &&
                  catalog.activeModelId === model.id;
                return (
                  <article
                    className={active ? "model-detail active" : "model-detail"}
                    key={model.id}
                  >
                    <div className="model-fields">
                      <label>
                        Display Name
                        <input
                          value={model.displayName}
                          onChange={(event) =>
                            updateProvider((current) => ({
                              ...current,
                              models: current.models.map((item) =>
                                item.id === model.id
                                  ? { ...item, displayName: event.target.value }
                                  : item,
                              ),
                            }))
                          }
                        />
                      </label>
                      <label>
                        Model ID
                        <input
                          value={model.modelId}
                          placeholder="provider-model-id"
                          onChange={(event) =>
                            updateProvider((current) => ({
                              ...current,
                              models: current.models.map((item) =>
                                item.id === model.id
                                  ? { ...item, modelId: event.target.value }
                                  : item,
                              ),
                            }))
                          }
                        />
                      </label>
                    </div>
                    <div className="model-actions">
                      <label className="inline-toggle">
                        <input
                          type="checkbox"
                          checked={model.enabled}
                          onChange={(event) =>
                            updateProvider((current) => ({
                              ...current,
                              models: current.models.map((item) =>
                                item.id === model.id
                                  ? { ...item, enabled: event.target.checked }
                                  : item,
                              ),
                            }))
                          }
                        />
                        Enabled
                      </label>
                      <button
                        type="button"
                        disabled={
                          active ||
                          !provider.enabled ||
                          !model.enabled ||
                          !model.modelId.trim()
                        }
                        onClick={() => void activate(provider, model)}
                      >
                        {active ? "✓ Active" : "Set Active"}
                      </button>
                      <button
                        type="button"
                        className="danger-text"
                        onClick={() =>
                          updateProvider((current) => ({
                            ...current,
                            models: current.models.filter(
                              (item) => item.id !== model.id,
                            ),
                          }))
                        }
                      >
                        Delete
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>
            <div className="provider-actions">
              <span>Keys remain in process memory only.</span>
              <button
                type="button"
                disabled={saving}
                onClick={() => void persist(provider)}
              >
                {saving ? "Saving…" : "Save Provider"}
              </button>
            </div>
          </>
        )}
      </div>
    </section>
  );
}
