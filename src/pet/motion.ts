import type { PetPosition, PetSize, WorkArea } from "./screen";

const DEFAULT_WANDER_DELAY_MS = 8_000;
const DEFAULT_MOVE_DURATION_MS = 2_400;
const FRAME_DELAY_MS = 32;

export function legalPositionBounds(
  workArea: WorkArea,
  petSize: PetSize = {
    width: workArea.petWidth,
    height: workArea.petHeight,
  },
) {
  return {
    minX: workArea.x,
    maxX: Math.max(workArea.x, workArea.x + workArea.width - petSize.width),
    minY: workArea.y,
    maxY: Math.max(workArea.y, workArea.y + workArea.height - petSize.height),
  };
}

export function randomValidPosition(
  workArea: WorkArea,
  petSize: PetSize = {
    width: workArea.petWidth,
    height: workArea.petHeight,
  },
  random = Math.random,
): PetPosition {
  const bounds = legalPositionBounds(workArea, petSize);
  return {
    x: Math.round(bounds.minX + random() * (bounds.maxX - bounds.minX)),
    y: Math.round(bounds.minY + random() * (bounds.maxY - bounds.minY)),
  };
}

export function interpolatePosition(
  from: PetPosition,
  to: PetPosition,
  progress: number,
): PetPosition {
  const clamped = Math.min(1, Math.max(0, progress));
  const eased =
    clamped < 0.5 ? 4 * clamped ** 3 : 1 - (-2 * clamped + 2) ** 3 / 2;
  return {
    x: Math.round(from.x + (to.x - from.x) * eased),
    y: Math.round(from.y + (to.y - from.y) * eased),
  };
}

type WanderLoopOptions = {
  workArea: WorkArea;
  initialPosition: PetPosition;
  move: (target: PetPosition) => Promise<unknown>;
  onMoveStart?: () => Promise<unknown>;
  onMoveFinished?: (target: PetPosition) => Promise<unknown>;
  signal: AbortSignal;
  wanderDelayMs?: number;
  moveDurationMs?: number;
  canMove?: () => Promise<boolean>;
};

function delay(milliseconds: number, signal: AbortSignal) {
  return new Promise<void>((resolve) => {
    const timer = globalThis.setTimeout(resolve, milliseconds);
    signal.addEventListener(
      "abort",
      () => {
        globalThis.clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
  });
}

export async function runWanderLoop({
  workArea,
  initialPosition,
  move,
  onMoveStart,
  onMoveFinished,
  signal,
  wanderDelayMs = DEFAULT_WANDER_DELAY_MS,
  moveDurationMs = DEFAULT_MOVE_DURATION_MS,
  canMove,
}: WanderLoopOptions) {
  let current = initialPosition;

  while (!signal.aborted) {
    await delay(wanderDelayMs, signal);
    if (signal.aborted) break;
    if (canMove && !(await canMove())) continue;

    const target = randomValidPosition(workArea);
    const origin = current;
    let lastPosition = current;
    const startedAt = performance.now();
    await onMoveStart?.();

    while (!signal.aborted) {
      const progress = (performance.now() - startedAt) / moveDurationMs;
      const next = interpolatePosition(origin, target, progress);
      await move(next);
      lastPosition = next;
      if (progress >= 1) break;
      await delay(FRAME_DELAY_MS, signal);
    }

    current = signal.aborted ? lastPosition : target;
    await onMoveFinished?.(current);
  }
}
