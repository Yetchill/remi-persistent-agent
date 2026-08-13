import { describe, expect, it } from "vitest";
import { AgentResponseValidationError, parseAgentResponse } from "./actions";

describe("structured Agent response parsing", () => {
  it("parses a valid AgentResponse JSON object", () => {
    const parsed = parseAgentResponse(
      JSON.stringify({
        intent: "respond",
        actions: [{ type: "speak", text: "你好。" }],
      }),
    );
    expect(parsed.stage).toBe("strict_json");
    expect(parsed.envelope.actions).toEqual([
      { type: "speak", text: "你好。" },
    ]);
  });

  it("extracts JSON from a json code fence", () => {
    const parsed = parseAgentResponse(
      '```json\n{"intent":"respond","actions":[{"type":"speak","text":"在这里。"}]}\n```',
    );
    expect(parsed.stage).toBe("fenced_json");
    expect(parsed.fallbackUsed).toBe(false);
  });

  it("deterministically extracts one JSON object from surrounding text", () => {
    const parsed = parseAgentResponse(
      'Here is the result: {"intent":"respond","actions":[{"type":"speak","text":"完成。"}]}',
    );
    expect(parsed.stage).toBe("extracted_json");
    expect(parsed.envelope.actions[0]).toEqual({
      type: "speak",
      text: "完成。",
    });
  });

  it("safely accepts a plain Chinese response beginning with 我", () => {
    const parsed = parseAgentResponse("我是 Remi，很高兴见到你。");
    expect(parsed.fallbackUsed).toBe(true);
    expect(parsed.envelope.actions).toEqual([
      { type: "speak", text: "我是 Remi，很高兴见到你。" },
    ]);
  });

  it("uses a safe generic response for malformed JSON", () => {
    const parsed = parseAgentResponse('{"actions":[{"type":"speak"}');
    expect(parsed.stage).toBe("safe_text_fallback");
    expect(parsed.envelope.actions[0]).toEqual({
      type: "speak",
      text: "我刚才没有组织好回答，可以再问我一次吗？",
    });
  });

  it("rejects a parsed action outside the validator whitelist", () => {
    expect(() => parseAgentResponse('{"actions":[{"type":"shell"}]}')).toThrow(
      AgentResponseValidationError,
    );
  });

  it("never creates side-effect actions in safe text fallback", () => {
    const parsed = parseAgentResponse("今天想聊点什么？");
    expect(parsed.envelope.actions).toHaveLength(1);
    expect(parsed.envelope.actions[0].type).toBe("speak");
  });
});
