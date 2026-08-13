import manifest from "../assets/pets/remi_hanfu/manifest.json";
import idle1 from "../assets/pets/remi_hanfu/idle_1.png";
import idle2 from "../assets/pets/remi_hanfu/idle_2.png";
import idle3 from "../assets/pets/remi_hanfu/idle_3.png";
import sleep from "../assets/pets/remi_hanfu/sleep.png";
import talk1 from "../assets/pets/remi_hanfu/talk_1.png";
import talk2 from "../assets/pets/remi_hanfu/talk_2.png";
import talk3 from "../assets/pets/remi_hanfu/talk_3.png";
import think from "../assets/pets/remi_hanfu/think.png";
import type { PetVisualState } from "./animation";

export const REMI_HANFU_ASSET_BY_FILENAME: Readonly<Record<string, string>> = {
  "idle_1.png": idle1,
  "idle_2.png": idle2,
  "idle_3.png": idle3,
  "talk_1.png": talk1,
  "talk_2.png": talk2,
  "talk_3.png": talk3,
  "think.png": think,
  "sleep.png": sleep,
};

function resolveFrames(filenames: readonly string[]) {
  return filenames.map((filename) => {
    const asset = REMI_HANFU_ASSET_BY_FILENAME[filename];
    if (!asset) throw new Error(`Unknown Remi character frame: ${filename}`);
    return asset;
  });
}

export const REMI_HANFU_CHARACTER = {
  name: manifest.name,
  displayName: manifest.displayName,
  defaultState: manifest.defaultState as PetVisualState,
  states: {
    idle: resolveFrames(manifest.states.idle),
    talk: resolveFrames(manifest.states.talk),
    think: resolveFrames(manifest.states.think),
    sleep: resolveFrames(manifest.states.sleep),
  },
  loops: {
    idle: resolveFrames(manifest.suggestedLoops.idle),
    talk: resolveFrames(manifest.suggestedLoops.talk),
  },
} as const;

export const REMI_HANFU_FRAMES = [
  ...new Set(Object.values(REMI_HANFU_ASSET_BY_FILENAME)),
];
