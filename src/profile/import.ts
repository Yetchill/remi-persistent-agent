import { invoke } from "@tauri-apps/api/core";
import type { ProfileImportResult, ProfilePreview } from "./types";

export function readCompanionProfileFile(file: File) {
  return file.text();
}

/** Parses and validates locally selected JSON without changing current state. */
export function previewCompanionProfile(profileJson: string) {
  return invoke<ProfilePreview>("preview_companion_profile", { profileJson });
}

/**
 * Replaces the portable profile after explicit confirmation. Rust creates a
 * readable backup before applying the import in a SQLite transaction.
 */
export function importCompanionProfile(
  profileJson: string,
  confirmReplace: boolean,
) {
  return invoke<ProfileImportResult>("import_companion_profile", {
    profileJson,
    confirmReplace,
  });
}
