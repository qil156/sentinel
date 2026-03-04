use anyhow::{anyhow, Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::types::{
    AssistantResponse, OpenAiContentItem, OpenAiInputItem, OpenAiJsonSchemaFormat, OpenAiRequest, OpenAiTextConfig,
    ScreenContext,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";
const MODEL_NAME: &str = "gpt-5.3-codex";

pub async fn ask_openai(question: &str, context: &ScreenContext) -> Result<AssistantResponse> {
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY is not set.")?;
    let client = reqwest::Client::new();

    let payload = OpenAiRequest {
        model: MODEL_NAME.to_string(),
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
                        text: format!(
                            "Window title: {}\n\nUser question: {}\n\nUse only the screenshot and the user question as context. Do not claim to have clicked, typed, or changed anything on the computer.",
                            context.window_title, question
                        ),
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
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body.".to_string());
        return Err(anyhow!("OpenAI request failed with status {status}: {body}"));
    }

    let body: Value = response.json().await.context("OpenAI response JSON was invalid.")?;
    extract_structured_response(&body)
}

fn extract_structured_response(body: &Value) -> Result<AssistantResponse> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return serde_json::from_str(text).context("Model output did not match the required JSON schema.");
    }

    if let Some(outputs) = body.get("output").and_then(Value::as_array) {
        for output in outputs {
            if let Some(contents) = output.get("content").and_then(Value::as_array) {
                for content in contents {
                    if let Some(text) = content.get("text").and_then(Value::as_str) {
                        if let Ok(parsed) = serde_json::from_str::<AssistantResponse>(text) {
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
            "screen_summary": {
                "type": "string"
            },
            "answer": {
                "type": "string"
            },
            "suggested_next_steps": {
                "type": "array",
                "items": {
                    "type": "string"
                }
            },
            "questions_to_clarify": {
                "type": "array",
                "items": {
                    "type": "string"
                }
            },
            "confidence": {
                "type": "number",
                "minimum": 0,
                "maximum": 1
            }
        }
    })
}

fn system_prompt() -> &'static str {
    "You are Sentinel, a Windows desktop assistant that answers questions about the current screen. You only observe the provided screenshot and window title. You never take actions. Return a JSON object that matches the provided schema. Keep the answer factual, concise, and grounded in visible UI evidence. If the screenshot is unclear, lower confidence and add clarifying questions."
}
