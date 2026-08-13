export type LlmMessage = {
  role: "system" | "user" | "assistant";
  content: string;
};

export type AgentLlmRequest = {
  eventId: string;
  eventType: string;
  messages: LlmMessage[];
  traceMetadata?: {
    provider?: string;
    model?: string;
    soulVersion: number;
    memoryPolicyVersion: string;
  };
};

export type AgentLlmResponse = {
  content: string;
  inputTokens?: number;
  outputTokens?: number;
};

export interface LlmProvider {
  complete(request: AgentLlmRequest): Promise<AgentLlmResponse>;
}

export interface EmbeddingProvider {
  embed(texts: string[]): Promise<number[][]>;
}

export interface ImageProvider {
  generate(request: unknown): Promise<unknown>;
}

export interface TtsProvider {
  synthesize(request: unknown): Promise<unknown>;
}
