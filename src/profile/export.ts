import { invoke } from "@tauri-apps/api/core";
import type { ProfileExportResult } from "./types";

/**
 * Writes a readable format-v1 JSON profile. When no destination is supplied,
 * Remi uses its app-data profile-exports directory and returns the exact path.
 */
export function exportCompanionProfile(destinationPath?: string) {
  return invoke<ProfileExportResult>("export_companion_profile", {
    destinationPath,
  });
}
