import { invoke } from "@tauri-apps/api/core";
import type {
  Memory,
  MemoryDeleteResult,
  MemoryDetail,
  MemoryFilter,
  UserMemoryEdit,
  MemoryViewerSnapshot,
  MemoryWriteResult,
} from "./types";

export function writeMemory(
  content: string,
  sourceRef: string,
  sourceType:
    | "user_explicit"
    | "agent_inferred"
    | "conversation"
    | "heartbeat"
    | "reflection"
    | "system",
) {
  return invoke<MemoryWriteResult>("write_memory", {
    candidate: { content, sourceRef, sourceType },
  });
}

export function getMemoryViewer(filter?: MemoryFilter, operationLimit = 100) {
  return invoke<MemoryViewerSnapshot>("get_memory_viewer", {
    filter,
    operationLimit,
  });
}

export function listMemories(filter?: MemoryFilter) {
  return getMemoryViewer(filter);
}

export function searchMemories(
  query: string,
  filter: Omit<MemoryFilter, "query"> = {},
) {
  return getMemoryViewer({ ...filter, query });
}

export function getMemoryDetail(memoryId: string) {
  return invoke<MemoryDetail>("get_memory_detail", { memoryId });
}

export function consolidateMemories() {
  return invoke<number>("consolidate_memories");
}

export function archiveMemory(memoryId: string) {
  return invoke<Memory>("archive_memory", { memoryId });
}

export function editMemory(memoryId: string, edit: UserMemoryEdit) {
  return invoke<Memory>("edit_memory", { memoryId, edit });
}

export function restoreMemory(memoryId: string) {
  return invoke<Memory>("restore_memory", { memoryId });
}

export function deleteMemory(memoryId: string) {
  return invoke<MemoryDeleteResult>("delete_memory", { memoryId });
}

export function pinMemory(memoryId: string) {
  return invoke<Memory>("pin_memory", { memoryId });
}
