import type { PetActivity, PetMood } from "../pet/petState";

export type AgentAction =
  | { type: "speak"; text: string }
  | {
      type: "remember";
      content: string;
      sourceType?: "user_explicit" | "agent_inferred";
    }
  | { type: "wander" }
  | { type: "sleep" }
  | { type: "wake" }
  | { type: "set_activity"; activity: PetActivity }
  | { type: "set_goal"; goal: string }
  | { type: "set_mood"; mood: PetMood }
  | { type: "noop" };

export type AgentActionEnvelope = {
  intent?: string;
  actions: AgentAction[];
};

export type AgentResponseParseStage =
  "strict_json" | "fenced_json" | "extracted_json" | "safe_text_fallback";

export type ParsedAgentResponse = {
  envelope: AgentActionEnvelope;
  stage: AgentResponseParseStage;
  fallbackUsed: boolean;
  parseError?: string;
};

export class AgentResponseValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AgentResponseValidationError";
  }
}

const MOODS: PetMood[] = ["neutral", "happy", "sleepy", "curious", "sad"];
const ACTIVITIES: PetActivity[] = [
  "idle",
  "wandering",
  "sleeping",
  "talking",
  "thinking",
];
const MALFORMED_RESPONSE_TEXT = "我刚才没有组织好回答，可以再问我一次吗？";

function validateEnvelope(value: unknown): AgentActionEnvelope {
  if (!value || typeof value !== "object" || !("actions" in value)) {
    throw new AgentResponseValidationError(
      "LLM output must contain an actions array",
    );
  }
  const candidate = value as { intent?: unknown; actions: unknown };
  if (!Array.isArray(candidate.actions) || candidate.actions.length === 0) {
    throw new AgentResponseValidationError(
      "LLM output must contain at least one action",
    );
  }
  const actions = candidate.actions.map((action): AgentAction => {
    if (!action || typeof action !== "object" || !("type" in action)) {
      throw new AgentResponseValidationError("Invalid action object");
    }
    const record = action as Record<string, unknown>;
    if (record.type === "noop") return { type: "noop" };
    if (record.type === "wander") return { type: "wander" };
    if (record.type === "sleep") return { type: "sleep" };
    if (record.type === "wake") return { type: "wake" };
    if (
      record.type === "speak" &&
      typeof record.text === "string" &&
      record.text.trim()
    ) {
      return { type: "speak", text: record.text.trim().slice(0, 4_000) };
    }
    if (
      record.type === "remember" &&
      typeof record.content === "string" &&
      record.content.trim()
    ) {
      return {
        type: "remember",
        content: record.content.trim().slice(0, 2_000),
        sourceType:
          record.sourceType === "user_explicit" ||
          record.sourceType === "agent_inferred"
            ? record.sourceType
            : undefined,
      };
    }
    if (
      record.type === "set_activity" &&
      typeof record.activity === "string" &&
      ACTIVITIES.includes(record.activity as PetActivity)
    ) {
      return { type: "set_activity", activity: record.activity as PetActivity };
    }
    if (
      record.type === "set_mood" &&
      typeof record.mood === "string" &&
      MOODS.includes(record.mood as PetMood)
    ) {
      return { type: "set_mood", mood: record.mood as PetMood };
    }
    if (
      record.type === "set_goal" &&
      typeof record.goal === "string" &&
      record.goal.trim()
    ) {
      return { type: "set_goal", goal: record.goal.trim().slice(0, 500) };
    }
    throw new AgentResponseValidationError(
      `Action is not allowed: ${String(record.type)}`,
    );
  });
  return {
    intent:
      typeof candidate.intent === "string"
        ? candidate.intent.slice(0, 80)
        : undefined,
    actions,
  };
}

function fencedJson(raw: string) {
  return raw.match(/```(?:json)?\s*([\s\S]*?)```/i)?.[1]?.trim();
}

function firstJsonObject(raw: string) {
  const start = raw.indexOf("{");
  if (start < 0) return undefined;
  let depth = 0;
  let quoted = false;
  let escaped = false;
  for (let index = start; index < raw.length; index += 1) {
    const character = raw[index];
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') quoted = false;
      continue;
    }
    if (character === '"') quoted = true;
    else if (character === "{") depth += 1;
    else if (character === "}") {
      depth -= 1;
      if (depth === 0) return raw.slice(start, index + 1);
    }
  }
  return undefined;
}

function tryJson(candidate: string | undefined) {
  if (!candidate) return { error: "No JSON candidate found" } as const;
  try {
    return { value: JSON.parse(candidate) as unknown } as const;
  } catch (error) {
    return {
      error: error instanceof Error ? error.message : "Invalid JSON",
    } as const;
  }
}

function safeFallbackText(raw: string) {
  const text = raw.trim();
  if (!text || text.startsWith("{") || text.includes("```")) {
    return MALFORMED_RESPONSE_TEXT;
  }
  return text.slice(0, 4_000);
}

export function parseAgentResponse(raw: string): ParsedAgentResponse {
  const trimmed = raw.trim();
  const errors: string[] = [];
  const attempts: Array<[AgentResponseParseStage, string | undefined]> = [
    ["strict_json", trimmed],
    ["fenced_json", fencedJson(trimmed)],
  ];
  const fenced = attempts[1][1];
  attempts.push([
    "extracted_json",
    firstJsonObject(fenced ? trimmed.replace(fenced, "") : trimmed),
  ]);

  for (const [stage, candidate] of attempts) {
    const result = tryJson(candidate);
    if ("value" in result) {
      return {
        envelope: validateEnvelope(result.value),
        stage,
        fallbackUsed: false,
      };
    }
    errors.push(`${stage}: ${result.error}`);
  }

  return {
    envelope: validateEnvelope({
      intent: "safe_text_fallback",
      actions: [{ type: "speak", text: safeFallbackText(raw) }],
    }),
    stage: "safe_text_fallback",
    fallbackUsed: true,
    parseError: errors.join(" | ").slice(0, 1_000),
  };
}
