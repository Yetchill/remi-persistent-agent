import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";

export type ModelConfig = {
  id: string;
  displayName: string;
  modelId: string;
  enabled: boolean;
  persisted?: boolean;
};

export type ProviderConfig = {
  id: string;
  displayName: string;
  type: "openai-compatible";
  baseUrl: string;
  enabled: boolean;
  hasApiKey: boolean;
  models: ModelConfig[];
  persisted?: boolean;
};

export type ProviderCatalog = {
  providers: ProviderConfig[];
  activeProviderId?: string;
  activeModelId?: string;
};

export type ProviderPreset = "openai" | "deepseek" | "qwen" | "kimi" | "custom";

const PRESETS: Record<
  ProviderPreset,
  { displayName: string; baseUrl: string }
> = {
  openai: { displayName: "OpenAI", baseUrl: "https://api.openai.com/v1" },
  deepseek: { displayName: "DeepSeek", baseUrl: "https://api.deepseek.com" },
  qwen: {
    displayName: "Qwen",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
  },
  kimi: { displayName: "Kimi", baseUrl: "https://api.moonshot.cn/v1" },
  custom: { displayName: "Custom Provider", baseUrl: "https://example.com/v1" },
};

export const PROVIDER_CATALOG_CHANGED = "provider-catalog-changed";

export function createProviderFromPreset(
  preset: ProviderPreset,
): ProviderConfig {
  const values = PRESETS[preset];
  return {
    id: crypto.randomUUID(),
    displayName: values.displayName,
    type: "openai-compatible",
    baseUrl: values.baseUrl,
    enabled: true,
    hasApiKey: false,
    persisted: false,
    models: [],
  };
}

function normalizeCatalog(catalog: ProviderCatalog): ProviderCatalog {
  return {
    ...catalog,
    providers: catalog.providers.map((provider) => ({
      ...provider,
      persisted: true,
      models: provider.models.map((model) => ({ ...model, persisted: true })),
    })),
  };
}

async function publishCatalog(promise: Promise<ProviderCatalog>) {
  const catalog = normalizeCatalog(await promise);
  await emit(PROVIDER_CATALOG_CHANGED, catalog);
  return catalog;
}

export function getProviderCatalog() {
  return invoke<ProviderCatalog>("get_provider_catalog").then(normalizeCatalog);
}

export function saveProvider(provider: ProviderConfig, apiKey = "") {
  return publishCatalog(
    invoke<ProviderCatalog>("save_provider", {
      input: { provider, apiKey },
    }),
  );
}

export function deleteProvider(providerId: string) {
  return publishCatalog(
    invoke<ProviderCatalog>("delete_provider", { providerId }),
  );
}

export function setActiveModel(providerId: string, modelId: string) {
  return publishCatalog(
    invoke<ProviderCatalog>("set_active_model", { providerId, modelId }),
  );
}

export function enabledModels(catalog?: ProviderCatalog) {
  if (!catalog) return [];
  return catalog.providers.flatMap((provider) =>
    provider.enabled
      ? provider.models
          .filter((model) => model.enabled)
          .map((model) => ({ provider, model }))
      : [],
  );
}
