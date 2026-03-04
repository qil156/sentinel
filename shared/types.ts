export type AssistantResponse = {
  screen_summary: string;
  answer: string;
  suggested_next_steps: string[];
  questions_to_clarify: string[];
  confidence: number;
};

export type ChatMessage =
  | {
      id: string;
      role: "user";
      text: string;
    }
  | {
      id: string;
      role: "assistant";
      response: AssistantResponse;
    };
