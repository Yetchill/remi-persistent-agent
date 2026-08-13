import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  activatePetPack,
  importPetPack,
  listPetPacks,
  onPetPackChanged,
  releasePetPack,
  resolvePetPack,
  type PetPack,
  type PetPackCatalog,
  type ResolvedPetPack,
  BUILTIN_PET_PACK_ID,
} from "./packs";
import { PetArtwork } from "./PetArtwork";

export function PetPackPanel({
  petName,
  opacity,
  onError,
  previewMode = false,
}: {
  petName: string;
  opacity: number;
  onError: (message?: string) => void;
  previewMode?: boolean;
}) {
  const [catalog, setCatalog] = useState<PetPackCatalog>();
  const [selectedId, setSelectedId] = useState<string>();
  const [preview, setPreview] = useState<ResolvedPetPack>();
  const [folderPath, setFolderPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [draggingFolder, setDraggingFolder] = useState(false);
  const previewRef = useRef<ResolvedPetPack | undefined>(undefined);

  const load = useCallback(async () => {
    const next = previewMode ? PREVIEW_CATALOG : await listPetPacks();
    setCatalog(next);
    setSelectedId((current) => current ?? next.activePetPackId);
    return next;
  }, [previewMode]);

  useEffect(() => {
    void load().catch((error: unknown) => onError(readError(error)));
    let disposed = false;
    let unlisten: (() => void) | undefined;
    if (!previewMode) {
      void onPetPackChanged(() => {
        void load().catch((error: unknown) => onError(readError(error)));
      }).then((next) => {
        if (disposed) next();
        else unlisten = next;
      });
    }
    return () => {
      disposed = true;
      unlisten?.();
      if (previewRef.current) releasePetPack(previewRef.current);
    };
  }, [load, onError, previewMode]);

  useEffect(() => {
    const selected = catalog?.packs.find((pack) => pack.id === selectedId);
    if (!selected) return;
    let cancelled = false;
    void resolvePetPack(selected).then(
      (resolved) => {
        if (cancelled) {
          releasePetPack(resolved);
          return;
        }
        if (previewRef.current) releasePetPack(previewRef.current);
        previewRef.current = resolved;
        setPreview(resolved);
      },
      (error: unknown) => onError(readError(error)),
    );
    return () => {
      cancelled = true;
    };
  }, [catalog, onError, selectedId]);

  useEffect(() => {
    if (previewMode) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "enter") {
          setDraggingFolder(true);
        } else if (payload.type === "leave") {
          setDraggingFolder(false);
        } else if (payload.type === "drop") {
          setDraggingFolder(false);
          const path = payload.paths[0];
          if (path) setFolderPath(path);
        }
      })
      .then((next) => {
        if (disposed) next();
        else unlisten = next;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [previewMode]);

  async function activate(pack: PetPack) {
    setBusy(true);
    onError(undefined);
    try {
      const next = await activatePetPack(pack.id);
      setCatalog(next);
      setSelectedId(pack.id);
    } catch (error) {
      onError(readError(error));
    } finally {
      setBusy(false);
    }
  }

  async function importFolder() {
    const path = folderPath.trim();
    if (!path) return;
    setBusy(true);
    onError(undefined);
    try {
      const imported = await importPetPack(path);
      const next = await load();
      setCatalog(next);
      setSelectedId(imported.id);
      setFolderPath("");
    } catch (error) {
      onError(readError(error));
    } finally {
      setBusy(false);
    }
  }

  const selected = catalog?.packs.find((pack) => pack.id === selectedId);
  const active = catalog?.packs.find(
    (pack) => pack.id === catalog.activePetPackId,
  );

  return (
    <div className="pet-pack-panel">
      <div className="section-heading">
        <div>
          <h3>Pet</h3>
          <p>
            Appearance only. Switching a Pet Pack never changes Remi's identity.
          </p>
        </div>
        <span className="pet-pack-current">
          Current: {active?.name ?? "Loading…"}
        </span>
      </div>

      <div className="pet-pack-browser">
        <div className="pet-pack-list" aria-label="Installed Pet Packs">
          {(catalog?.packs ?? []).map((pack) => (
            <button
              type="button"
              key={pack.id}
              className={selectedId === pack.id ? "selected" : ""}
              onClick={() => setSelectedId(pack.id)}
            >
              <span>{pack.name}</span>
              <small>
                {pack.source === "builtin" ? "Built in" : "Imported"} · v
                {pack.version}
              </small>
              {catalog?.activePetPackId === pack.id && (
                <strong>✓ Active</strong>
              )}
            </button>
          ))}
        </div>

        <div className="pet-pack-preview">
          {preview && (
            <PetArtwork
              name={petName}
              opacity={opacity}
              visualState="idle"
              pack={preview}
            />
          )}
          <strong>{selected?.name ?? "Select a Pet Pack"}</strong>
          <button
            type="button"
            disabled={
              busy || !selected || selected.id === catalog?.activePetPackId
            }
            onClick={() => selected && void activate(selected)}
          >
            {selected?.id === catalog?.activePetPackId
              ? "Active"
              : "Use This Pet"}
          </button>
        </div>
      </div>

      <div className="pet-pack-import">
        <label>
          Import Pet Pack Folder
          <input
            value={folderPath}
            placeholder="/Users/you/Desktop/my-pet-pack"
            onChange={(event) => setFolderPath(event.target.value)}
          />
        </label>
        <button
          type="button"
          disabled={busy || !folderPath.trim()}
          onClick={() => void importFolder()}
        >
          Validate & Import
        </button>
      </div>
      <div className={`pet-pack-dropzone ${draggingFolder ? "active" : ""}`}>
        Or drop a Pet Pack folder into this window
      </div>
      <p className="settings-note">
        Folder import checks manifest.json, at least one idle PNG, and every
        referenced PNG. Missing talk, think, or sleep states fall back to idle.
      </p>
    </div>
  );
}

const PREVIEW_PACK: PetPack = {
  id: BUILTIN_PET_PACK_ID,
  name: "Remi Hanfu",
  version: "1.0",
  defaultState: "idle",
  source: "builtin",
  rootPath: null,
  states: {
    idle: ["idle_1.png", "idle_2.png", "idle_3.png"],
    talk: ["talk_1.png", "talk_2.png", "talk_3.png"],
    think: ["think.png"],
    sleep: ["sleep.png"],
  },
  suggestedLoops: {
    idle: ["idle_1.png", "idle_2.png", "idle_3.png", "idle_2.png"],
    talk: ["talk_1.png", "talk_2.png", "talk_3.png", "talk_2.png"],
  },
};

const PREVIEW_CATALOG: PetPackCatalog = {
  activePetPackId: BUILTIN_PET_PACK_ID,
  packs: [PREVIEW_PACK],
};

function readError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
