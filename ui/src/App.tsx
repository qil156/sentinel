import { FormEvent, KeyboardEvent, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AssistantResponse, ChatMessage } from "../../shared/types";

export function App() {
  const formRef = useRef<HTMLFormElement>(null);
  const [question, setQuestion] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const trimmed = question.trim();
    if (!trimmed || isLoading) {
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

  return (
    <main className="shell">
      <section className="widget">
        <header className="widget-header">
          <div>
            <p className="eyebrow">Screen-aware assistant</p>
            <h1>Sentinel</h1>
          </div>
          <span className="status">{isLoading ? "Analyzing" : "Idle"}</span>
        </header>

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
          <button disabled={isLoading || question.trim().length === 0} type="submit">
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
