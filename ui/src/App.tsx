import { FormEvent, KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AssistantResponse,
  ChatMessage,
  ConversationContext,
  ConversationTurn,
  ProviderModelOption,
  UserLlmSettings
} from "../../shared/types";

type Conversation = {
  id: string;
  title: string;
  messages: ChatMessage[];
  memory: ConversationMemory;
  createdAt: number;
  updatedAt: number;
};

type ConversationMemory = {
  conversationSummary: string;
  taskGoal: string;
  currentPage: string;
  knownFacts: string[];
  openQuestions: string[];
  lastRecommendedSteps: string[];
};

function createEmptyMemory(): ConversationMemory {
  return {
    conversationSummary: "",
    taskGoal: "",
    currentPage: "",
    knownFacts: [],
    openQuestions: [],
    lastRecommendedSteps: []
  };
}

function createConversation(): Conversation {
  const now = Date.now();
  return {
    id: crypto.randomUUID(),
    title: "New chat",
    messages: [],
    memory: createEmptyMemory(),
    createdAt: now,
    updatedAt: now
  };
}

function conversationTitleFromQuestion(question: string): string {
  const cleaned = question.trim().replace(/\s+/g, " ");
  if (!cleaned) {
    return "New chat";
  }
  return cleaned.length > 36 ? `${cleaned.slice(0, 36)}...` : cleaned;
}

function chatMessageToTurn(message: ChatMessage): ConversationTurn {
  if (message.role === "user") {
    return {
      role: "user",
      content: message.text
    };
  }

  return {
    role: "assistant",
    content: [
      `Screen summary: ${message.response.screen_summary}`,
      `Answer: ${message.response.answer}`,
      message.response.suggested_next_steps.length > 0
        ? `Suggested next steps: ${message.response.suggested_next_steps.join("; ")}`
        : "",
      message.response.questions_to_clarify.length > 0
        ? `Questions to clarify: ${message.response.questions_to_clarify.join("; ")}`
        : ""
    ]
      .filter(Boolean)
      .join("\n")
  };
}

function extractKeyFacts(answer: string): string[] {
  return answer
    .split(/(?<=[.!?])\s+/)
    .map((part) => part.trim())
    .filter(Boolean)
    .slice(0, 3);
}

function dedupePreserveOrder(items: string[]): string[] {
  const seen = new Set<string>();
  const output: string[] = [];
  for (const item of items) {
    const normalized = item.trim();
    if (!normalized) {
      continue;
    }
    const key = normalized.toLowerCase();
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    output.push(normalized);
  }
  return output;
}

function buildConversationSummary(
  previousSummary: string,
  userQuestion: string,
  response: AssistantResponse
): string {
  const sections = [
    previousSummary.trim(),
    `User asked: ${userQuestion.trim()}`,
    response.screen_summary ? `Current page: ${response.screen_summary}` : "",
    response.answer ? `Latest answer: ${response.answer}` : ""
  ]
    .filter(Boolean)
    .join("\n");

  return sections.length > 700 ? `${sections.slice(sections.length - 700)}` : sections;
}

function updateConversationMemory(
  current: ConversationMemory,
  userQuestion: string,
  response: AssistantResponse
): ConversationMemory {
  const taskGoal = current.taskGoal || userQuestion.trim();
  const currentPage = response.screen_summary.trim() || current.currentPage;
  const knownFacts = dedupePreserveOrder([
    currentPage,
    ...current.knownFacts,
    ...extractKeyFacts(response.answer)
  ]).slice(0, 8);

  return {
    conversationSummary: buildConversationSummary(current.conversationSummary, userQuestion, response),
    taskGoal,
    currentPage,
    knownFacts,
    openQuestions: dedupePreserveOrder(response.questions_to_clarify).slice(0, 5),
    lastRecommendedSteps: dedupePreserveOrder(response.suggested_next_steps).slice(0, 5)
  };
}

function buildConversationContext(conversation: Conversation): ConversationContext {
  const messages = conversation.messages;
  const turns = messages.map(chatMessageToTurn);
  const recentTurnLimit = 8;
  const recentMessages = turns.slice(-recentTurnLimit);

  return {
    conversation_summary: conversation.memory.conversationSummary,
    task_goal: conversation.memory.taskGoal,
    current_page: conversation.memory.currentPage,
    known_facts: conversation.memory.knownFacts,
    open_questions: conversation.memory.openQuestions,
    last_recommended_steps: conversation.memory.lastRecommendedSteps,
    recent_messages: recentMessages
  };
}

export function App() {
  const formRef = useRef<HTMLFormElement>(null);
  const messagesRef = useRef<HTMLDivElement>(null);
  const [question, setQuestion] = useState("");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [options, setOptions] = useState<ProviderModelOption[]>([]);
  const [settings, setSettings] = useState<UserLlmSettings | null>(null);
  const [isSavingApiKey, setIsSavingApiKey] = useState(false);
  const [showApiKeyPanel, setShowApiKeyPanel] = useState(false);
  const [conversations, setConversations] = useState<Conversation[]>(() => [createConversation()]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [showHistoryPanel, setShowHistoryPanel] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [loadingConversationId, setLoadingConversationId] = useState<string | null>(null);
  const [animatedAssistantMessageId, setAnimatedAssistantMessageId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const providerOptions = useMemo(() => {
    const map = new Map<string, { id: string; label: string; available: boolean }>();
    options.forEach((opt) => {
      if (!map.has(opt.provider_id)) {
        map.set(opt.provider_id, {
          id: opt.provider_id,
          label: opt.provider_label,
          available: opt.is_available
        });
      } else if (opt.is_available) {
        const current = map.get(opt.provider_id)!;
        map.set(opt.provider_id, { ...current, available: true });
      }
    });
    return Array.from(map.values());
  }, [options]);

  const modelOptions = useMemo(
    () => options.filter((opt) => opt.provider_id === settings?.selected_provider),
    [options, settings?.selected_provider]
  );
  const activeConversation = useMemo(
    () => conversations.find((conversation) => conversation.id === activeConversationId) ?? conversations[0] ?? null,
    [conversations, activeConversationId]
  );
  const activeMessages = activeConversation?.messages ?? [];

  useEffect(() => {
    async function boot() {
      try {
        const [catalog, userSettings] = await Promise.all([
          invoke<ProviderModelOption[]>("get_model_options"),
          invoke<UserLlmSettings>("get_user_llm_settings")
        ]);
        setOptions(catalog);
        setSettings(userSettings);
        setShowApiKeyPanel(!userSettings.has_selected_provider_key);
      } catch (invokeError) {
        const message = toDisplayError(
          invokeError instanceof Error
            ? invokeError.message
            : "Could not load model settings."
        );
        setError(message);
      }
    }

    void boot();
  }, []);

  useEffect(() => {
    if (!activeConversationId && conversations.length > 0) {
      setActiveConversationId(conversations[0].id);
    }
  }, [activeConversationId, conversations]);

  useEffect(() => {
    scrollMessagesToBottom();
  }, [activeMessages]);

  useEffect(() => {
    if (!activeConversationId) {
      return;
    }
    const raf = window.requestAnimationFrame(() => {
      scrollMessagesToBottom();
    });
    return () => window.cancelAnimationFrame(raf);
  }, [activeConversationId]);

  useEffect(() => {
    if (!isLoading) {
      return;
    }
    scrollMessagesToBottom();
  }, [isLoading]);

  function scrollMessagesToBottom() {
    const container = messagesRef.current;
    if (!container) {
      return;
    }
    container.scrollTop = container.scrollHeight;
  }

  async function sendQuestion(rawQuestion: string) {
    const trimmed = rawQuestion.trim();
    if (!trimmed || isLoading || !settings?.has_selected_provider_key || !activeConversation) {
      return;
    }

    setError(null);
    setIsLoading(true);
    setLoadingConversationId(activeConversation.id);

    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      text: trimmed
    };

    const conversationId = activeConversation.id;
    setConversations((current) =>
      current.map((conversation) => {
        if (conversation.id !== conversationId) {
          return conversation;
        }
        const nextMessages = [...conversation.messages, userMessage];
        return {
          ...conversation,
          title:
            conversation.messages.length === 0
              ? conversationTitleFromQuestion(trimmed)
              : conversation.title,
          messages: nextMessages,
          updatedAt: Date.now()
        };
      })
    );
    setQuestion("");

    try {
      const conversationContext = buildConversationContext(activeConversation);
      const response = await invoke<AssistantResponse>("ask_about_screen", {
        question: trimmed,
        conversationContext
      });

      const assistantMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        response
      };
      setConversations((current) =>
        current.map((conversation) => {
          if (conversation.id !== conversationId) {
            return conversation;
          }
          return {
            ...conversation,
            messages: [...conversation.messages, assistantMessage],
            memory: updateConversationMemory(conversation.memory, trimmed, response),
            updatedAt: Date.now()
          };
        })
      );
      setAnimatedAssistantMessageId(assistantMessage.id);
    } catch (invokeError) {
      const message = toDisplayError(
        invokeError instanceof Error
          ? invokeError.message
          : typeof invokeError === "string"
            ? invokeError
            : `Sentinel could not process the current screen: ${JSON.stringify(invokeError)}`
      );
      setError(message);
    } finally {
      setIsLoading(false);
      setLoadingConversationId(null);
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await sendQuestion(question);
  }

  function handleNewChat() {
    if (isLoading) {
      return;
    }
    if (activeConversation && activeConversation.messages.length === 0) {
      setQuestion("");
      setError(null);
      setShowHistoryPanel(false);
      return;
    }
    const created = createConversation();
    setConversations((current) => [created, ...current]);
    setActiveConversationId(created.id);
    setQuestion("");
    setError(null);
    setShowHistoryPanel(false);
  }

  function handleSelectConversation(conversationId: string) {
    if (activeConversation && activeConversation.messages.length === 0 && activeConversation.id !== conversationId) {
      setConversations((current) =>
        current.filter((conversation) => conversation.id !== activeConversation.id)
      );
    }
    setAnimatedAssistantMessageId(null);
    setActiveConversationId(conversationId);
    setShowHistoryPanel(false);
    setError(null);
  }

  async function handleProviderChange(provider: string) {
    const firstModel = options.find((opt) => opt.provider_id === provider);
    if (!firstModel) {
      return;
    }
    await updateModelSelection(provider, firstModel.model_id);
  }

  async function handleModelChange(model: string) {
    if (!settings) {
      return;
    }
    await updateModelSelection(settings.selected_provider, model);
  }

  async function updateModelSelection(provider: string, model: string) {
    setError(null);
    try {
      const updated = await invoke<UserLlmSettings>("set_model_selection", {
        provider,
        model
      });
      setSettings(updated);
      setShowApiKeyPanel(!updated.has_selected_provider_key);
    } catch (invokeError) {
      const message = toDisplayError(
        invokeError instanceof Error
          ? invokeError.message
          : "Could not update model selection."
      );
      setError(message);
    }
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      formRef.current?.requestSubmit();
    }
  }

  async function handleSaveApiKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!settings) {
      return;
    }

    const trimmed = apiKeyInput.trim();
    if (!trimmed || isSavingApiKey) {
      return;
    }

    setError(null);
    setIsSavingApiKey(true);
    try {
      await invoke("save_api_key", {
        provider: settings.selected_provider,
        api_key: trimmed,
        apiKey: trimmed
      });

      const refreshed = await invoke<UserLlmSettings>("get_user_llm_settings");
      setSettings(refreshed);
      setShowApiKeyPanel(false);
      setApiKeyInput("");
    } catch (invokeError) {
      const message = toDisplayError(
        invokeError instanceof Error
          ? invokeError.message
          : typeof invokeError === "string"
            ? invokeError
            : `Could not save API key: ${JSON.stringify(invokeError)}`
      );
      setError(message);
    } finally {
      setIsSavingApiKey(false);
    }
  }

  const historyItems = useMemo(
    () =>
      conversations
        .filter((conversation) => conversation.messages.length > 0)
        .sort((a, b) => b.updatedAt - a.updatedAt),
    [conversations]
  );

  return (
    <main className="shell">
      <section className="widget">
        <header className="widget-header">
          <div>
            <p className="eyebrow">Screen-aware assistant</p>
            <h1>Sentinel</h1>
          </div>
          <div className="header-actions">
            <div className="history-wrap">
              <button
                className="api-key-toggle"
                onClick={() => setShowHistoryPanel((open) => !open)}
                type="button"
              >
                History
              </button>
              {showHistoryPanel ? (
                <div className="history-panel">
                  {historyItems.length === 0 ? (
                    <p className="history-empty">No chats yet.</p>
                  ) : (
                    historyItems.map((conversation) => (
                      <button
                        className={`history-item ${conversation.id === activeConversation?.id ? "active" : ""}`}
                        key={conversation.id}
                        onClick={() => handleSelectConversation(conversation.id)}
                        type="button"
                      >
                        {conversation.title}
                      </button>
                    ))
                  )}
                </div>
              ) : null}
            </div>
            <button className="api-key-toggle" disabled={isLoading} onClick={handleNewChat} type="button">
              New Chat
            </button>
            <button className="api-key-toggle" onClick={() => setShowApiKeyPanel((v) => !v)} type="button">
              Settings
            </button>
            <span className="status">{isLoading ? "Analyzing" : "Idle"}</span>
          </div>
        </header>

        {showApiKeyPanel ? (
          <section className="api-key-panel">
            <h2>Model & API Key</h2>
            <p>Choose provider/model and save the API key for the selected provider.</p>
            <div className="model-grid">
              <label>
                Provider
                <select
                  value={settings?.selected_provider ?? ""}
                  onChange={(event) => void handleProviderChange(event.target.value)}
                >
                  {providerOptions.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.label}
                      {provider.available ? "" : " (Coming soon)"}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Model
                <select
                  value={settings?.selected_model ?? ""}
                  onChange={(event) => void handleModelChange(event.target.value)}
                >
                  {modelOptions.map((model) => (
                    <option key={`${model.provider_id}:${model.model_id}`} value={model.model_id}>
                      {model.model_label}
                      {model.is_available ? "" : " (Coming soon)"}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <form onSubmit={handleSaveApiKey}>
              <input
                aria-label="Provider API key"
                placeholder={`${settings?.selected_provider ?? "provider"} API key`}
                type="password"
                value={apiKeyInput}
                onChange={(event) => setApiKeyInput(event.target.value)}
              />
              <button disabled={isSavingApiKey || apiKeyInput.trim().length === 0} type="submit">
                {isSavingApiKey ? "Saving..." : "Save Key"}
              </button>
            </form>
          </section>
        ) : null}

        <div className="messages" ref={messagesRef}>
          {activeMessages.length === 0 ? (
            <div className="empty-state">
              Ask about the active window. Sentinel will capture the foreground window and answer using the visible
              screen context.
            </div>
          ) : (
            activeMessages.map((message) =>
              message.role === "user" ? (
                <article className="bubble bubble-user" key={message.id}>
                  {message.text}
                </article>
              ) : (
                <article className="bubble bubble-assistant" key={message.id}>
                  <StructuredResponseCard
                    response={message.response}
                    animate={message.id === animatedAssistantMessageId}
                  />
                </article>
              )
            )
          )}
          {isLoading && loadingConversationId === activeConversation?.id ? (
            <article className="bubble bubble-assistant loading-bubble">
              <div className="loading-dots" aria-label="Loading answer" role="status">
                <span />
                <span />
                <span />
              </div>
            </article>
          ) : null}
        </div>

        {error ? <div className="error-banner">{error}</div> : null}

        <form className="composer" onSubmit={handleSubmit} ref={formRef}>
          <textarea
            aria-label="Ask Sentinel about the current screen"
            placeholder="What is on this screen, and what should I do next?"
            rows={3}
            value={question}
            onChange={(event) => setQuestion(event.target.value)}
            onKeyDown={handleComposerKeyDown}
          />
          <button disabled={isLoading || !settings?.has_selected_provider_key || question.trim().length === 0} type="submit">
            Send
          </button>
        </form>
      </section>
    </main>
  );
}

function toDisplayError(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) {
    return "Unexpected error.";
  }

  const compact = trimmed.replace(/\s+/g, " ");
  const maxLength = 320;
  if (compact.length <= maxLength) {
    return compact;
  }

  return `${compact.slice(0, maxLength)}...`;
}

function StructuredResponseCard({
  response,
  animate
}: {
  response: AssistantResponse;
  animate: boolean;
}) {
  const [animatedSummary, setAnimatedSummary] = useState("");
  const [animatedAnswer, setAnimatedAnswer] = useState("");
  const [visibleStepCount, setVisibleStepCount] = useState(0);
  const [visibleQuestionCount, setVisibleQuestionCount] = useState(0);
  const [animatedConfidence, setAnimatedConfidence] = useState(0);

  useEffect(() => {
    const summary = response.screen_summary ?? "";
    const full = response.answer ?? "";
    const steps = response.suggested_next_steps ?? [];
    const questions = response.questions_to_clarify ?? [];
    const targetConfidence = Math.round((response.confidence ?? 0) * 100);

    if (!animate) {
      setAnimatedSummary(summary);
      setAnimatedAnswer(full);
      setVisibleStepCount(steps.length);
      setVisibleQuestionCount(questions.length);
      setAnimatedConfidence(targetConfidence);
      return;
    }

    setAnimatedSummary("");
    setAnimatedAnswer("");
    setVisibleStepCount(0);
    setVisibleQuestionCount(0);
    setAnimatedConfidence(0);

    if (!summary && !full && steps.length === 0 && questions.length === 0) {
      return;
    }

    let summaryIndex = 0;
    let answerIndex = 0;
    let stepIndex = 0;
    let questionIndex = 0;
    let confidenceValue = 0;
    const summaryStep = Math.max(1, Math.ceil(summary.length / 40));
    const answerStep = Math.max(1, Math.ceil(full.length / 90));
    let phase: "summary" | "answer" | "steps" | "questions" | "confidence" | "done" = "summary";

    const timer = window.setInterval(() => {
      if (phase === "summary") {
        summaryIndex = Math.min(summary.length, summaryIndex + summaryStep);
        setAnimatedSummary(summary.slice(0, summaryIndex));
        if (summaryIndex >= summary.length) {
          phase = "answer";
        }
        return;
      }

      if (phase === "answer") {
        answerIndex = Math.min(full.length, answerIndex + answerStep);
        setAnimatedAnswer(full.slice(0, answerIndex));
        if (answerIndex >= full.length) {
          phase = "steps";
        }
        return;
      }

      if (phase === "steps") {
        if (stepIndex < steps.length) {
          stepIndex += 1;
          setVisibleStepCount(stepIndex);
        } else {
          phase = "questions";
        }
        return;
      }

      if (phase === "questions") {
        if (questionIndex < questions.length) {
          questionIndex += 1;
          setVisibleQuestionCount(questionIndex);
        } else {
          phase = "confidence";
        }
        return;
      }

      if (phase === "confidence") {
        confidenceValue = Math.min(targetConfidence, confidenceValue + 4);
        setAnimatedConfidence(confidenceValue);
        if (confidenceValue >= targetConfidence) {
          phase = "done";
          window.clearInterval(timer);
        }
        return;
      }
    }, 24);

    return () => window.clearInterval(timer);
  }, [response, animate]);

  return (
    <div className="response-card">
      <section>
        <h2>Screen Summary</h2>
        <p>{animatedSummary}</p>
      </section>
      <section>
        <h2>Answer</h2>
        <p>{animatedAnswer}</p>
      </section>
      <section>
        <h2>Suggested Next Steps</h2>
        {response.suggested_next_steps.length > 0 ? (
          <ul>
            {response.suggested_next_steps.slice(0, visibleStepCount).map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ul>
        ) : (
          <p>None</p>
        )}
      </section>
      <section>
        <h2>Questions To Clarify</h2>
        {response.questions_to_clarify.length > 0 ? (
          <ul>
            {response.questions_to_clarify.slice(0, visibleQuestionCount).map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        ) : (
          <p>None</p>
        )}
      </section>
      <section className="confidence">
        <h2>Confidence</h2>
        <p>{animatedConfidence}%</p>
      </section>
    </div>
  );
}
