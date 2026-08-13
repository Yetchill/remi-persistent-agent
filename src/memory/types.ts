export type MemoryKind = "semantic" | "episodic" | "relationship";
export type MemoryStatus = "active" | "outdated" | "archived" | "merged";
export type MemorySourceType =
  | "user_explicit"
  | "user_correction"
  | "agent_inferred"
  | "conversation"
  | "heartbeat"
  | "reflection"
  | "system";

export type Memory = {
  id: string;
  kind: MemoryKind;
  content: string;
  importance: number;
  confidence: number;
  status: MemoryStatus;
  createdAt: number;
  updatedAt: number;
  lastAccessedAt?: number;
  accessCount: number;
  validFrom?: number;
  validTo?: number;
  sourceType?: MemorySourceType;
  sourceRef?: string;
  pinned: boolean;
};

export type MemoryWriteResult = {
  decision: "ADD" | "UPDATE" | "MERGE" | "SUPERSEDE" | "IGNORE";
  reason: string;
  memory?: Memory;
  lifecycle: {
    operation: MemoryWriteResult["decision"];
    memoryType: MemoryKind;
    content: string;
    metadata: Record<string, unknown>;
    relatedMemoryIds: string[];
    reasonLabel: string;
  };
};

export type MemoryRelation = {
  sourceId: string;
  targetId: string;
  relation:
    "supports" | "contradicts" | "supersedes" | "derived_from" | "merged_into";
  createdAt: number;
};

export type MemoryOperation = {
  id: string;
  memoryId?: string;
  operation:
    | "ADD"
    | "UPDATE"
    | "MERGE"
    | "SUPERSEDE"
    | "IGNORE"
    | "RETRIEVE"
    | "CONSOLIDATE"
    | "USER_EDIT"
    | "ARCHIVE"
    | "RESTORE"
    | "USER_DELETE"
    | "PIN";
  timestamp: number;
  sourceEventId?: string;
  reasonLabel?: string;
  relatedMemoryIdsJson?: string;
  detailJson?: string;
};

export type MemoryViewerSnapshot = {
  memories: Memory[];
  relations: MemoryRelation[];
  operations: MemoryOperation[];
  counts: MemoryStatusCounts;
  policyVersion: string;
};

export type MemoryStatusCounts = {
  active: number;
  outdated: number;
  archived: number;
  merged: number;
  total: number;
};

export type MemoryFilter = {
  kind?: MemoryKind;
  status?: MemoryStatus;
  query?: string;
};

export type MemoryDetail = {
  memory: Memory;
  relations: MemoryRelation[];
  operations: MemoryOperation[];
};

export type UserMemoryEdit = {
  content: string;
  kind?: MemoryKind;
  importance?: number;
  confidence?: number;
};

export type MemoryDeleteResult = {
  deletedId: string;
  relationsDeleted: number;
  operationsDeleted: number;
};
