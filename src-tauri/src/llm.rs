use anyhow::{anyhow, Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::types::{
    AssistantResponse, OpenAiContentItem, OpenAiInputItem, OpenAiJsonSchemaFormat, OpenAiRequest, OpenAiTextConfig,
    ScreenContext,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/chat/completions";

pub async fn ask_with_provider(
    provider: &str,
    model: &str,
    question: &str,
    context: &ScreenContext,
    api_key: &str,
) -> Result<AssistantResponse> {
    match provider {
        "openai" => ask_openai(model, question, context, api_key).await,
        "anthropic" => ask_anthropic(model, question, context, api_key).await,
        "google" => ask_gemini(model, question, context, api_key).await,
        "deepseek" => ask_deepseek(model, question, context, api_key).await,
        _ => Err(anyhow!("Unsupported provider: {provider}")),
    }
}

async fn ask_openai(model: &str, question: &str, context: &ScreenContext, api_key: &str) -> Result<AssistantResponse> {
    let client = reqwest::Client::new();

    let payload = OpenAiRequest {
        model: model.to_string(),
        input: vec![
            OpenAiInputItem {
                role: "system".to_string(),
                content: vec![OpenAiContentItem::InputText {
                    text: system_prompt().to_string(),
                }],
            },
            OpenAiInputItem {
                role: "user".to_string(),
                content: vec![
                    OpenAiContentItem::InputText {
                        text: user_prompt(question, &context.window_title),
                    },
                    OpenAiContentItem::InputImage {
                        image_url: format!("data:image/png;base64,{}", context.image_base64),
                    },
                ],
            },
        ],
        text: OpenAiTextConfig {
            format: OpenAiJsonSchemaFormat {
                kind: "json_schema".to_string(),
                name: "sentinel_screen_answer".to_string(),
                strict: true,
                schema: response_schema(),
            },
        },
    };

    let response = client
        .post(OPENAI_API_URL)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await
        .context("OpenAI request failed.")?;

    if !response.status().is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "OpenAI request failed.".to_string());
        return Err(anyhow!(extract_error_message(&body)));
    }

    let body: Value = response.json().await.context("OpenAI response JSON was invalid.")?;
    extract_structured_response(&body)
}

async fn ask_anthropic(
    model: &str,
    question: &str,
    context: &ScreenContext,
    api_key: &str,
) -> Result<AssistantResponse> {
    let client = reqwest::Client::new();
    let payload = json!({
      "model": model,
      "max_tokens": 1200,
      "system": system_prompt(),
      "messages": [{
        "role": "user",
        "content": [
          { "type": "text", "text": user_prompt(question, &context.window_title) + "\n\nReturn ONLY valid JSON that matches the schema." },
          { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": context.image_base64 } }
        ]
      }]
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await
        .context("Anthropic request failed.")?;

    if !response.status().is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Anthropic request failed.".to_string());
        return Err(anyhow!(extract_error_message(&body)));
    }

    let body: Value = response
        .json()
        .await
        .context("Anthropic response JSON was invalid.")?;
    let text = body
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                let kind = item.get("type").and_then(Value::as_str)?;
                if kind == "text" {
                    item.get("text").and_then(Value::as_str)
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| anyhow!("Anthropic did not return text content."))?;

    parse_json_response_text(text)
}

async fn ask_gemini(model: &str, question: &str, context: &ScreenContext, api_key: &str) -> Result<AssistantResponse> {
    let client = reqwest::Client::new();
    let url = format!("{GEMINI_API_BASE}/{model}:generateContent?key={api_key}");
    let payload = json!({
      "systemInstruction": {
        "parts": [{ "text": system_prompt() }]
      },
      "contents": [{
        "role": "user",
        "parts": [
          { "text": user_prompt(question, &context.window_title) },
          { "inline_data": { "mime_type": "image/png", "data": context.image_base64 } }
        ]
      }],
      "generationConfig": {
        "responseMimeType": "application/json",
        "responseSchema": gemini_response_schema()
      }
    });

    let response = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await
        .context("Gemini request failed.")?;

    if !response.status().is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Gemini request failed.".to_string());
        return Err(anyhow!(extract_error_message(&body)));
    }

    let body: Value = response.json().await.context("Gemini response JSON was invalid.")?;
    let text = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .and_then(|parts| parts.first())
        .and_then(|part| part.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Gemini did not return text content."))?;

    parse_json_response_text(text)
}

async fn ask_deepseek(model: &str, question: &str, context: &ScreenContext, api_key: &str) -> Result<AssistantResponse> {
    let client = reqwest::Client::new();
    let payload = json!({
      "model": model,
      "response_format": { "type": "json_object" },
      "messages": [
        { "role": "system", "content": system_prompt() },
        {
          "role": "user",
          "content": format!(
            "{}\n\nNote: DeepSeek model '{}' in this build is called without image attachment, so screen analysis confidence should be lower.",
            user_prompt(question, &context.window_title),
            model
          )
        }
      ]
    });

    let response = client
        .post(DEEPSEEK_API_URL)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await
        .context("DeepSeek request failed.")?;

    if !response.status().is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "DeepSeek request failed.".to_string());
        return Err(anyhow!(extract_error_message(&body)));
    }

    let body: Value = response
        .json()
        .await
        .context("DeepSeek response JSON was invalid.")?;
    let text = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("DeepSeek did not return message content."))?;

    parse_json_response_text(text)
}

fn parse_json_response_text(text: &str) -> Result<AssistantResponse> {
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return assistant_response_from_value(value);
    }

    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                let candidate = &text[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                    return assistant_response_from_value(value);
                }
            }
        }
    }

    Ok(fallback_response_from_text(text))
}

fn extract_structured_response(body: &Value) -> Result<AssistantResponse> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return parse_json_response_text(text);
    }

    if let Some(outputs) = body.get("output").and_then(Value::as_array) {
        for output in outputs {
            if let Some(contents) = output.get("content").and_then(Value::as_array) {
                for content in contents {
                    if let Some(text) = content.get("text").and_then(Value::as_str) {
                        if let Ok(parsed) = parse_json_response_text(text) {
                            return Ok(parsed);
                        }
                    }
                }
            }
        }
    }

    Err(anyhow!("Could not find a structured JSON result in the model response."))
}

fn response_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "screen_summary",
            "answer",
            "suggested_next_steps",
            "questions_to_clarify",
            "confidence"
        ],
        "properties": {
            "screen_summary": { "type": "string" },
            "answer": { "type": "string" },
            "suggested_next_steps": {
                "type": "array",
                "items": { "type": "string" }
            },
            "questions_to_clarify": {
                "type": "array",
                "items": { "type": "string" }
            },
            "confidence": {
                "type": "number",
                "minimum": 0,
                "maximum": 1
            }
        }
    })
}

fn gemini_response_schema() -> Value {
    json!({
        "type": "OBJECT",
        "required": [
            "screen_summary",
            "answer",
            "suggested_next_steps",
            "questions_to_clarify",
            "confidence"
        ],
        "properties": {
            "screen_summary": { "type": "STRING" },
            "answer": { "type": "STRING" },
            "suggested_next_steps": {
                "type": "ARRAY",
                "items": { "type": "STRING" }
            },
            "questions_to_clarify": {
                "type": "ARRAY",
                "items": { "type": "STRING" }
            },
            "confidence": { "type": "NUMBER" }
        }
    })
}

fn system_prompt() -> &'static str {
    "You are Sentinel, a Windows desktop assistant that answers questions about the current screen. You only observe the provided screenshot and window title. You never take actions. Return JSON matching the required schema.\n\nFormatting rules:\n- screen_summary: one short sentence naming only the current page type (for example: email inbox, CI pipeline page, spreadsheet, chart dashboard).\n- answer: concise and factual, grounded only in visible UI evidence.\n- suggested_next_steps: direct imperative actions the user can do next. Do NOT use assistant-offer language such as 'if you want, I can...'.\n- questions_to_clarify: only include if needed.\n- confidence: 0 to 1."
}

fn user_prompt(question: &str, window_title: &str) -> String {
    format!(
        "Window title: {window_title}\n\nUser question: {question}\n\nUse only the screenshot and the user question as context. Do not claim to have clicked, typed, or changed anything on the computer."
    )
}

fn extract_error_message(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(message) = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
        {
            return message.to_string();
        }
        if let Some(message) = value
            .get("error")
            .and_then(|error| error.get("details"))
            .and_then(Value::as_array)
            .and_then(|details| details.first())
            .and_then(|item| item.get("message"))
            .and_then(Value::as_str)
        {
            return message.to_string();
        }
        if let Some(message) = value
            .get("message")
            .and_then(Value::as_str)
        {
            return message.to_string();
        }
    }

    body.trim().to_string()
}

fn assistant_response_from_value(value: Value) -> Result<AssistantResponse> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("Model output did not match the required JSON schema."))?;

    let screen_summary = value_to_string(obj.get("screen_summary"));
    let answer = value_to_string(obj.get("answer"));
    let suggested_next_steps = value_to_string_list(obj.get("suggested_next_steps"));
    let questions_to_clarify = value_to_string_list(obj.get("questions_to_clarify"));
    let confidence = value_to_confidence(obj.get("confidence"));

    let clean_summary = if screen_summary.is_empty() {
        "The model returned an unstructured response.".to_string()
    } else {
        normalize_screen_summary(&screen_summary)
    };
    let clean_answer = if answer.is_empty() {
        "I could not parse a strict JSON answer from this provider response. Please retry once or switch to a different model.".to_string()
    } else {
        answer
    };

    Ok(AssistantResponse {
        screen_summary: clean_summary,
        answer: clean_answer,
        suggested_next_steps: normalize_suggested_next_steps(suggested_next_steps),
        questions_to_clarify,
        confidence,
    })
}

fn value_to_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn value_to_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(s) => s.trim().to_string(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

fn value_to_confidence(value: Option<&Value>) -> f32 {
    let parsed = match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
    .unwrap_or(0.2);

    parsed.clamp(0.0, 1.0) as f32
}

fn fallback_response_from_text(text: &str) -> AssistantResponse {
    let condensed = text
        .replace('\r', " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let preview = if condensed.is_empty() {
        "No response text was returned by the model.".to_string()
    } else {
        condensed.chars().take(800).collect::<String>()
    };

    AssistantResponse {
        screen_summary: "The model returned non-JSON output.".to_string(),
        answer: preview,
        suggested_next_steps: vec![
            "Retry once with the same model.".to_string(),
            "If it keeps happening, switch to a model with stronger JSON compliance.".to_string(),
        ],
        questions_to_clarify: vec![],
        confidence: 0.2,
    }
}

fn normalize_screen_summary(raw: &str) -> String {
    let mut compact = raw
        .replace('\r', " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if let Some(first_sentence) = compact.split('.').next() {
        compact = first_sentence.trim().to_string();
    }

    if compact.len() > 90 {
        compact.truncate(90);
        compact = compact.trim_end().to_string();
    }

    compact
}

fn normalize_suggested_next_steps(steps: Vec<String>) -> Vec<String> {
    let banned_prefixes = [
        "if you want",
        "if you'd like",
        "if you would like",
        "i can",
        "i could",
        "let me",
    ];

    steps
        .into_iter()
        .map(|step| {
            let mut normalized = step.trim().to_string();
            let lowered = normalized.to_lowercase();
            for prefix in banned_prefixes {
                if lowered.starts_with(prefix) {
                    normalized = normalized[prefix.len()..].trim_start_matches(|c: char| {
                        c == ',' || c == ':' || c == ';' || c == '-' || c.is_whitespace()
                    }).to_string();
                    break;
                }
            }
            normalized
        })
        .filter(|step| !step.is_empty())
        .collect()
}
