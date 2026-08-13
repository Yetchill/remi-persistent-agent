import { invoke } from "@tauri-apps/api/core";
import type { AgentLlmRequest, AgentLlmResponse, LlmProvider } from "../types";

export class OpenAiCompatibleProvider implements LlmProvider {
  complete(request: AgentLlmRequest) {
    return invoke<AgentLlmResponse>("complete_llm", { request });
  }
}
