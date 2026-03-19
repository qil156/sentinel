export type AssistantResponse = {
  screen_summary: string;
  answer: string;
  suggested_next_steps: string[];
  questions_to_clarify: string[];
  confidence: number;
};

export type ProviderModelOption = {
  provider_id: string;
  provider_label: string;
  model_id: string;
  model_label: string;
  is_available: boolean;
};

export type UserLlmSettings = {
  selected_provider: string;
  selected_model: string;
  has_selected_provider_key: boolean;
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

export type ConversationTurn = {
  role: "user" | "assistant";
  content: string;
};

export type ConversationContext = {
  conversation_summary: string;
  task_goal: string;
  current_page: string;
  known_facts: string[];
  open_questions: string[];
  last_recommended_steps: string[];
  recent_messages: ConversationTurn[];
};
