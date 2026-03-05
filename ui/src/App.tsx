import { FormEvent, KeyboardEvent, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AssistantResponse, ChatMessage } from "../../shared/types";

export function App() {
  const formRef = useRef<HTMLFormElement>(null);
  const [question, setQuestion] = useState("");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [hasApiKey, setHasApiKey] = useState<boolean | null>(null);
  const [isSavingApiKey, setIsSavingApiKey] = useState(false);
  const [showApiKeyPanel, setShowApiKeyPanel] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function checkApiKey() {
      try {
        const exists = await invoke<boolean>("has_api_key");
        setHasApiKey(exists);
        setShowApiKeyPanel(!exists);
      } catch {
        setHasApiKey(false);
        setShowApiKeyPanel(true);
      }
    }

    void checkApiKey();
  }, []);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const trimmed = question.trim();
    if (!trimmed || isLoading || !hasApiKey) {
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
      const message =
        invokeError instanceof Error
          ? invokeError.message
          : "Sentinel could not process the current screen.";
      setError(message);
    } finally {
      setIsLoading(false);
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

    const trimmed = apiKeyInput.trim();
    if (!trimmed || isSavingApiKey) {
      return;
    }

    setError(null);
    setIsSavingApiKey(true);
    try {
      await invoke("save_api_key", { api_key: trimmed, apiKey: trimmed });
      setHasApiKey(true);
      setShowApiKeyPanel(false);
      setApiKeyInput("");
    } catch (invokeError) {
      const message =
        invokeError instanceof Error
          ? invokeError.message
          : typeof invokeError === "string"
            ? invokeError
            : `Could not save API key: ${JSON.stringify(invokeError)}`;
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
              API Key
            </button>
            <span className="status">{isLoading ? "Analyzing" : "Idle"}</span>
          </div>
        </header>

        {showApiKeyPanel ? (
          <section className="api-key-panel">
            <h2>Set OpenAI API Key</h2>
            <p>Sentinel stores your key locally and uses it for all requests from this app.</p>
            <form onSubmit={handleSaveApiKey}>
              <input
                aria-label="OpenAI API key"
                placeholder="sk-..."
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

        <div className="messages">
          {messages.length === 0 ? (
            <div className="empty-state">
              Ask about the active window. Sentinel will capture the foreground window and answer using the visible screen context.
            </div>
          ) : (
            messages.map((message) =>
              message.role === "user" ? (
                <article className="bubble bubble-user" key={message.id}>
                  {message.text}
                </article>
              ) : (
                <article className="bubble bubble-assistant" key={message.id}>
                  <StructuredResponseCard response={message.response} />
                </article>
              )
            )
          )}
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
          <button disabled={isLoading || !hasApiKey || question.trim().length === 0} type="submit">
            Send
          </button>
        </form>
      </section>
    </main>
  );
}

function StructuredResponseCard({ response }: { response: AssistantResponse }) {
  return (
    <div className="response-card">
      <section>
        <h2>Screen Summary</h2>
        <p>{response.screen_summary}</p>
      </section>
      <section>
        <h2>Answer</h2>
        <p>{response.answer}</p>
      </section>
      <section>
        <h2>Suggested Next Steps</h2>
        {response.suggested_next_steps.length > 0 ? (
          <ul>
            {response.suggested_next_steps.map((step) => (
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
            {response.questions_to_clarify.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        ) : (
          <p>None</p>
        )}
      </section>
      <section className="confidence">
        <h2>Confidence</h2>
        <p>{Math.round(response.confidence * 100)}%</p>
      </section>
    </div>
  );
}
