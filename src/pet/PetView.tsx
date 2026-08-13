import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef } from "react";
import { PetArtwork } from "./PetArtwork";
import type { PetVisualState } from "./animation";
import type { ResolvedPetPack } from "./packs";

type PetViewProps = {
  name: string;
  opacity: number;
  visualState: PetVisualState;
  pack?: ResolvedPetPack;
  onClick: () => void;
  onContextMenu: () => void;
  onDragStart: () => void;
  onDragEnd: (position: { x: number; y: number }) => void;
  onDragCancel: () => void;
};

export async function readPositionAfterPointerRelease(
  readPosition: () => Promise<{ x: number; y: number }>,
  waitForFrame = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => resolve())),
) {
  await waitForFrame();
  return readPosition();
}

export function PetView({
  name,
  opacity,
  visualState,
  pack,
  onClick,
  onContextMenu,
  onDragStart,
  onDragEnd,
  onDragCancel,
}: PetViewProps) {
  const startRef = useRef<{ x: number; y: number } | undefined>(undefined);
  const draggingRef = useRef(false);
  const finishingRef = useRef(false);
  const suppressClickRef = useRef(false);

  async function finishNativeDrag() {
    if (!draggingRef.current || finishingRef.current) return;
    finishingRef.current = true;
    try {
      const position = await readPositionAfterPointerRelease(() =>
        getCurrentWindow().outerPosition(),
      );
      onDragEnd({ x: position.x, y: position.y });
    } catch {
      onDragCancel();
    } finally {
      draggingRef.current = false;
      finishingRef.current = false;
      startRef.current = undefined;
    }
  }

  useEffect(() => {
    const finish = () => void finishNativeDrag();
    window.addEventListener("pointerup", finish, true);
    window.addEventListener("mouseup", finish, true);
    return () => {
      window.removeEventListener("pointerup", finish, true);
      window.removeEventListener("mouseup", finish, true);
    };
  });

  function beginNativeDrag() {
    if (draggingRef.current) return;
    draggingRef.current = true;
    suppressClickRef.current = true;
    onDragStart();
    void getCurrentWindow()
      .startDragging()
      .catch(() => {
        draggingRef.current = false;
        startRef.current = undefined;
        onDragCancel();
      });
  }

  return (
    <button
      className="pet"
      type="button"
      aria-label={`和 ${name} 聊天`}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        startRef.current = { x: event.screenX, y: event.screenY };
      }}
      onPointerMove={(event) => {
        const start = startRef.current;
        if (!start || draggingRef.current) return;
        if (Math.hypot(event.screenX - start.x, event.screenY - start.y) >= 5) {
          beginNativeDrag();
        }
      }}
      onPointerUp={() => {
        if (draggingRef.current) {
          void finishNativeDrag();
        } else {
          startRef.current = undefined;
        }
      }}
      onClick={(event) => {
        if (suppressClickRef.current) {
          suppressClickRef.current = false;
          event.preventDefault();
          return;
        }
        onClick();
      }}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onContextMenu();
      }}
    >
      <PetArtwork
        name={name}
        opacity={opacity}
        visualState={visualState}
        pack={pack}
      />
    </button>
  );
}
