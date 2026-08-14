import { describe, expect, it } from "vitest";
import type { ProviderCatalog } from "./config";
import { reconcileProviderCatalogDraft } from "./ProvidersPanel";

const saved: ProviderCatalog = {
  providers: [
    {
      id: "provider-1",
      displayName: "OpenAI",
      type: "openai-compatible",
      baseUrl: "https://api.openai.com/v1",
      enabled: true,
      hasApiKey: false,
      models: [],
      persisted: true,
    },
  ],
  activeProviderId: "provider-1",
};

describe("Provider draft reconciliation", () => {
  it("does not overwrite an unsaved form when an external catalog arrives", () => {
    const draft: ProviderCatalog = {
      ...saved,
      providers: [
        {
          ...saved.providers[0],
          baseUrl: "https://copied.example.com/v1",
          persisted: false,
        },
      ],
    };

    const reconciled = reconcileProviderCatalogDraft(draft, saved, true);
    expect(reconciled.providers[0].baseUrl).toBe(
      "https://copied.example.com/v1",
    );
    expect(reconciled.activeProviderId).toBe("provider-1");
  });

  it("accepts persisted data when there is no local draft", () => {
    const stale: ProviderCatalog = { providers: [] };
    expect(reconcileProviderCatalogDraft(stale, saved, false)).toBe(saved);
  });
});
