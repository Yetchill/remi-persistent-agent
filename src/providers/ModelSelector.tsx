import { useMemo } from "react";
import { setActiveModel, type ProviderCatalog } from "./config";

type ModelSelectorProps = {
  catalog: ProviderCatalog;
  onChange: (catalog: ProviderCatalog) => void;
  onError?: (message?: string) => void;
  compact?: boolean;
};

export function ModelSelector({
  catalog,
  onChange,
  onError,
  compact = false,
}: ModelSelectorProps) {
  const enabledProviders = useMemo(
    () =>
      catalog.providers
        .filter((provider) => provider.enabled && provider.persisted !== false)
        .map((provider) => ({
          provider,
          models: provider.models.filter(
            (model) =>
              model.enabled &&
              model.persisted !== false &&
              model.modelId.trim(),
          ),
        }))
        .filter((entry) => entry.models.length > 0),
    [catalog.providers],
  );
  const selected =
    catalog.activeProviderId && catalog.activeModelId
      ? `${catalog.activeProviderId}::${catalog.activeModelId}`
      : "";

  async function select(value: string) {
    const separator = value.indexOf("::");
    if (separator < 1) return;
    onError?.(undefined);
    try {
      onChange(
        await setActiveModel(
          value.slice(0, separator),
          value.slice(separator + 2),
        ),
      );
    } catch (error) {
      onError?.(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <select
      className={compact ? "model-selector compact" : "model-selector"}
      aria-label="Active Provider and Model"
      value={selected}
      onChange={(event) => void select(event.target.value)}
    >
      <option value="">Select Provider · Model</option>
      {enabledProviders.map(({ provider, models }) => (
        <optgroup label={provider.displayName} key={provider.id}>
          {models.map((model) => (
            <option key={model.id} value={`${provider.id}::${model.id}`}>
              {provider.displayName} · {model.displayName}
            </option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}
