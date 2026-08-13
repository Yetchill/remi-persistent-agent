import { invoke } from "@tauri-apps/api/core";
import type { Memory } from "./types";

export function retrieveMemories(query: string, limit = 6) {
  return invoke<Memory[]>("retrieve_memories", { query, limit });
}

export function getRelationshipSummary() {
  return invoke<string>("get_relationship_summary");
}
