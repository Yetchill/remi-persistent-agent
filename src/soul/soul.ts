import { invoke } from "@tauri-apps/api/core";

export type SoulDocument = { content: string; soulVersion: number };

export function getSoul() {
  return invoke<SoulDocument>("get_soul");
}

export function updateSoul(content: string) {
  return invoke<SoulDocument>("update_soul", { content });
}
