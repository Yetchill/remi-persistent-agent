import { useEffect } from "react";
import { usePetAnimationFrame, type PetVisualState } from "./animation";
import { REMI_HANFU_FRAMES } from "./assets";
import type { ResolvedPetPack } from "./packs";

type PetArtworkProps = {
  name: string;
  opacity: number;
  visualState?: PetVisualState;
  className?: string;
  pack?: ResolvedPetPack;
};

export function PetArtwork({
  name,
  opacity,
  visualState = "idle",
  className,
  pack,
}: PetArtworkProps) {
  const frame = usePetAnimationFrame(visualState, pack);
  useEffect(() => {
    for (const source of pack?.allFrames ?? REMI_HANFU_FRAMES) {
      const image = new Image();
      image.src = source;
    }
  }, [pack]);
  return (
    <img
      src={frame}
      className={className}
      draggable={false}
      alt={`${name} desktop pet`}
      style={{ opacity }}
    />
  );
}
