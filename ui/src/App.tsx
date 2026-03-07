import { FormEvent, KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AssistantResponse,
  ChatMessage,
  ProviderModelOption,
  UserLlmSettings
} from "../../shared/types";

export function App() {
  const formRef = useRef<HTMLFormElement>(null);
  const messagesRef = useRef<HTMLDivElement>(null);
  const [question, setQuestion] = useState("");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [options, setOptions] = useState<ProviderModelOption[]>([]);
  const [settings, setSettings] = useState<UserLlmSettings | null>(null);
  const [isSavingApiKey, setIsSavingApiKey] = useState(false);
  const [showApiKeyPanel, setShowApiKeyPanel] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
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
    scrollMessagesToBottom();
  }, [messages]);

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
    if (!trimmed || isLoading || !settings?.has_selected_provider_key) {
      return;
    }

    setError(null);
    setIsLoading(true);

    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      text: trimmed
    };

    setMessages((current) => [...current, userMessage]);
    setQuestion("");

    try {
      const response = await invoke<AssistantResponse>("ask_about_screen", {
        question: trimmed
      });

      setMessages((current) => [
        ...current,
        {
          id: crypto.randomUUID(),
          role: "assistant",
          response
        }
      ]);
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
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await sendQuestion(question);
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

  return (
    <main className="shell">
      <section className="widget">
        <header className="widget-header">
          <div>
            <p className="eyebrow">Screen-aware assistant</p>
            <h1>Sentinel</h1>
          </div>
          <div className="header-actions">
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
          {messages.length === 0 ? (
            <div className="empty-state">
              Ask about the active window. Sentinel will capture the foreground window and answer using the visible
              screen context.
            </div>
          ) : (
            messages.map((message) =>
              message.role === "user" ? (
                <article className="bubble bubble-user" key={message.id}>
                  {message.text}
                </article>
              ) : (
                <article className="bubble bubble-assistant" key={message.id}>
                  <StructuredResponseCard
                    response={message.response}
                  />
                </article>
              )
            )
          )}
          {isLoading ? (
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
  response
}: {
  response: AssistantResponse;
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
  }, [response]);

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
