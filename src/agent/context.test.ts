import { describe, expect, it } from "vitest";
import type { PetState } from "../pet/petState";
import { buildContext, type ConversationMessage } from "./context";
import { createEvent } from "./events";

const state: PetState = {
  energy: 80,
  boredom: 20,
  mood: "curious",
  activity: "thinking",
  x: 10,
  y: 20,
  opacity: 1,
};

describe("Context Builder", () => {
  it("includes Soul, state, event and only the latest twelve messages", () => {
    const history: ConversationMessage[] = Array.from(
      { length: 15 },
      (_, index) => ({
        id: String(index),
        role: index % 2 ? "assistant" : "user",
        content: `message-${index}`,
        timestamp: index,
      }),
    );
    const context = buildContext(
      createEvent("USER_MESSAGE", "user", { text: "hi" }),
      history,
      {
        soul: "Name: Remi",
        petState: state,
        runtimeMetadata: {
          activeProviderName: "DeepSeek",
          activeModelDisplayName: "DeepSeek Reasoner",
          activeModelId: "deepseek-reasoner",
        },
      },
    );

    expect(context[0].content).toContain("[SOUL]\nName: Remi");
    expect(context[0].content).toContain('"mood":"curious"');
    expect(context[0].content).toContain("USER_MESSAGE");
    expect(context[0].content).toContain("[AGENT IDENTITY]");
    expect(context[0].content).toContain("Provider = DeepSeek");
    expect(context[0].content).toContain("Model = DeepSeek Reasoner");
    expect(context[0].content).toContain("Model ID = deepseek-reasoner");
    expect(context).toHaveLength(13);
    expect(context[1].content).toBe("message-3");
  });

  it("places retrieved and relationship memories in labeled sections", () => {
    const context = buildContext(createEvent("USER_MESSAGE", "user"), [], {
      soul: "Name: Remi",
      petState: state,
      relevantMemories: ["[semantic] User's cat is named Cream"],
      relationshipSummary: "The user named the pet Remi",
    });
    expect(context[0].content).toContain("[RELEVANT MEMORIES]");
    expect(context[0].content).toContain("User's cat is named Cream");
    expect(context[0].content).toContain("[RELATIONSHIP SUMMARY]");
  });
});
