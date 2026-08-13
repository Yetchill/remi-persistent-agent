import { emitTo } from "@tauri-apps/api/event";
import { type ChangeEvent, useRef, useState } from "react";
import {
  exportCompanionProfile,
  importCompanionProfile,
  previewCompanionProfile,
  readCompanionProfileFile,
  type ProfileExportResult,
  type ProfileImportResult,
  type ProfilePreview,
} from ".";
import { PROFILE_IMPORTED } from "../windows/events";

export function ProfileControls({
  onError,
  onImported,
}: {
  onError: (message?: string) => void;
  onImported: () => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [profileJson, setProfileJson] = useState<string>();
  const [preview, setPreview] = useState<ProfilePreview>();
  const [exportResult, setExportResult] = useState<ProfileExportResult>();
  const [importResult, setImportResult] = useState<ProfileImportResult>();
  const [busy, setBusy] = useState(false);

  async function exportProfile() {
    setBusy(true);
    onError(undefined);
    try {
      setExportResult(await exportCompanionProfile());
    } catch (error) {
      onError(readError(error));
    } finally {
      setBusy(false);
    }
  }

  async function selectProfile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setBusy(true);
    onError(undefined);
    try {
      const json = await readCompanionProfileFile(file);
      const nextPreview = await previewCompanionProfile(json);
      setProfileJson(json);
      setPreview(nextPreview);
      setImportResult(undefined);
    } catch (error) {
      setProfileJson(undefined);
      setPreview(undefined);
      onError(readError(error));
    } finally {
      setBusy(false);
    }
  }

  async function replaceProfile() {
    if (!profileJson || !preview) return;
    if (
      !window.confirm(
        `Replace the current Remi profile with ${preview.agentName}? A local backup will be created first.`,
      )
    ) {
      return;
    }
    setBusy(true);
    onError(undefined);
    try {
      const result = await importCompanionProfile(profileJson, true);
      setImportResult(result);
      setProfileJson(undefined);
      setPreview(undefined);
      onImported();
      await Promise.all([
        emitTo("pet-window", PROFILE_IMPORTED),
        emitTo("settings-window", PROFILE_IMPORTED),
      ]);
    } catch (error) {
      onError(readError(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="profile-controls">
      <div className="section-heading">
        <div>
          <h3>Companion Profile</h3>
          <p>
            Back up identity, SOUL, memories, relationships, behavior, and Pet
            preference. Provider secrets are never included.
          </p>
        </div>
      </div>
      <div className="profile-actions">
        <button
          type="button"
          disabled={busy}
          onClick={() => void exportProfile()}
        >
          Export Profile
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => inputRef.current?.click()}
        >
          Import Profile
        </button>
        <input
          ref={inputRef}
          className="visually-hidden"
          type="file"
          accept="application/json,.json"
          onChange={(event) => void selectProfile(event)}
        />
      </div>

      {exportResult && (
        <p className="profile-result">
          Exported a readable profile to <code>{exportResult.path}</code>
        </p>
      )}

      {preview && (
        <div className="profile-preview">
          <strong>Import preview</strong>
          <dl>
            <dt>Agent</dt>
            <dd>{preview.agentName}</dd>
            <dt>SOUL version</dt>
            <dd>{preview.soulVersion}</dd>
            <dt>Memories</dt>
            <dd>{preview.memoryCount}</dd>
            <dt>Relationship state</dt>
            <dd>
              {preview.relationshipAvailable
                ? `Available (${preview.relationshipMemoryCount})`
                : "Not available"}
            </dd>
            <dt>Pet preference</dt>
            <dd>{preview.activePetPackId}</dd>
            <dt>Backend reference</dt>
            <dd>{preview.provider.modelName ?? "None"}</dd>
          </dl>
          <p>
            Import replaces the current portable profile only. Local Provider
            configuration, API keys, recent conversation, position, and traces
            remain unchanged.
          </p>
          <div className="profile-actions">
            <button
              type="button"
              disabled={busy}
              onClick={() => void replaceProfile()}
            >
              Replace Current Profile
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => {
                setProfileJson(undefined);
                setPreview(undefined);
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {importResult && (
        <p className="profile-result">
          Imported {importResult.preview.agentName}. Previous profile backup:{" "}
          <code>{importResult.backupPath}</code>
        </p>
      )}
    </div>
  );
}

function readError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
