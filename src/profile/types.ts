import type { Memory, MemoryRelation } from "../memory/types";
import type { AppSettings } from "../settings/settings";

export type ProfileProviderMetadata = {
  providerId?: string;
  providerName?: string;
  providerType?: string;
  modelId?: string;
  modelName?: string;
};

export type ProfileRelationshipState = {
  available: boolean;
  summary: string;
  memoryIds: string[];
};

export type CompanionProfile = {
  formatVersion: 1;
  agentName: string;
  soul: string;
  soulVersion: number;
  createdAt: number;
  memories: Memory[];
  relations: MemoryRelation[];
  relationshipState: ProfileRelationshipState;
  behavior: AppSettings;
  activePetPackId: string;
  provider: ProfileProviderMetadata;
};

export type ProfilePreview = {
  formatVersion: number;
  agentName: string;
  soulVersion: number;
  createdAt: number;
  memoryCount: number;
  relationshipAvailable: boolean;
  relationshipMemoryCount: number;
  activePetPackId: string;
  provider: ProfileProviderMetadata;
};

export type ProfileExportResult = {
  path: string;
  preview: ProfilePreview;
};

export type ProfileImportResult = {
  backupPath: string;
  preview: ProfilePreview;
};
