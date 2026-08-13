import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { REMI_HANFU_ASSET_BY_FILENAME } from "./assets";
import type { PetVisualState } from "./animation";

export const BUILTIN_PET_PACK_ID = "remi-hanfu-v1";
export const PET_PACK_CHANGED_EVENT = "pet-pack-changed";

export type PetPackSource = "builtin" | "imported";

export type PetPack = {
  id: string;
  name: string;
  version: string;
  defaultState: string;
  states: Record<string, string[]>;
  suggestedLoops: Record<string, string[]>;
  source: PetPackSource;
  /** Present only for validated copies below app-data/pet-packs. */
  rootPath: string | null;
};

export type PetPackCatalog = {
  activePetPackId: string;
  packs: PetPack[];
};

export type ResolvedPetPack = Omit<PetPack, "states" | "suggestedLoops"> & {
  states: Record<string, string[]>;
  suggestedLoops: Record<string, string[]>;
  allFrames: string[];
};

export function listPetPacks() {
  return invoke<PetPackCatalog>("list_pet_packs");
}

/**
 * Imports a validated copy of one folder. Import and activation stay separate so
 * Settings can preview a pack without changing Remi's current appearance.
 */
export function importPetPack(folderPath: string) {
  return invoke<PetPack>("import_pet_pack", { folderPath });
}

export function activatePetPack(petPackId: string) {
  return invoke<PetPackCatalog>("activate_pet_pack", { petPackId });
}

export function onPetPackChanged(
  listener: (pack: PetPack) => void,
): Promise<UnlistenFn> {
  return listen<PetPack>(PET_PACK_CHANGED_EVENT, ({ payload }) =>
    listener(payload),
  );
}

/** Always returns a non-empty frame list; unsupported states use idle. */
export function frameFilesForState(
  pack: Pick<PetPack, "states" | "suggestedLoops">,
  state: PetVisualState,
): string[] {
  const idle = pack.states.idle ?? [];
  const stateFrames = pack.states[state]?.length ? pack.states[state] : idle;
  if (state === "idle") {
    return pack.suggestedLoops.idle?.length
      ? pack.suggestedLoops.idle
      : stateFrames;
  }
  if (state === "talk") {
    return pack.suggestedLoops.talk?.length
      ? pack.suggestedLoops.talk
      : stateFrames;
  }
  return stateFrames;
}

async function resolveFrame(pack: PetPack, filename: string) {
  if (pack.source === "builtin") {
    return (
      REMI_HANFU_ASSET_BY_FILENAME[filename] ??
      REMI_HANFU_ASSET_BY_FILENAME[pack.states.idle?.[0] ?? "idle_1.png"]
    );
  }
  if (!pack.rootPath) {
    throw new Error(`Imported Pet Pack '${pack.id}' has no asset root`);
  }
  const payload = await invoke<ArrayBuffer | Uint8Array | number[]>(
    "read_pet_pack_frame",
    { petPackId: pack.id, filename },
  );
  const bytes =
    payload instanceof ArrayBuffer
      ? new Uint8Array(payload)
      : Uint8Array.from(payload);
  return URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
}

/** Resolves bundled imports or scoped app-data asset URLs for the renderer. */
export async function resolvePetPack(pack: PetPack): Promise<ResolvedPetPack> {
  const filenames = [
    ...new Set([
      ...Object.values(pack.states).flat(),
      ...Object.values(pack.suggestedLoops).flat(),
    ]),
  ];
  const resolvedResults = await Promise.allSettled(
    filenames.map((filename) => resolveFrame(pack, filename)),
  );
  const resolvedUrls = resolvedResults
    .filter(
      (result): result is PromiseFulfilledResult<string> =>
        result.status === "fulfilled",
    )
    .map((result) => result.value);
  const failure = resolvedResults.find(
    (result): result is PromiseRejectedResult => result.status === "rejected",
  );
  if (failure) {
    releasePetPack({ allFrames: resolvedUrls });
    throw failure.reason;
  }
  const urlByFilename = Object.fromEntries(
    filenames.map((filename, index) => [filename, resolvedUrls[index]]),
  );
  const resolveMap = (frameMap: Record<string, string[]>) =>
    Object.fromEntries(
      Object.entries(frameMap).map(([state, stateFilenames]) => [
        state,
        stateFilenames.map((filename) => urlByFilename[filename]),
      ]),
    );
  const states = resolveMap(pack.states);
  const suggestedLoops = resolveMap(pack.suggestedLoops);
  return {
    ...pack,
    states,
    suggestedLoops,
    allFrames: [
      ...new Set([
        ...Object.values(states).flat(),
        ...Object.values(suggestedLoops).flat(),
      ]),
    ],
  };
}

/** Revoke imported frame URLs after switching packs or unmounting the renderer. */
export function releasePetPack(pack: Pick<ResolvedPetPack, "allFrames">) {
  for (const frame of pack.allFrames) {
    if (frame.startsWith("blob:")) URL.revokeObjectURL(frame);
  }
}
