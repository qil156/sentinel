use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantResponse {
    pub screen_summary: String,
    pub answer: String,
    pub suggested_next_steps: Vec<String>,
    pub questions_to_clarify: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ScreenContext {
    pub window_title: String,
    pub image_base64: String,
}

#[derive(Debug, Serialize)]
pub struct OpenAiRequest {
    pub model: String,
    pub input: Vec<OpenAiInputItem>,
    pub text: OpenAiTextConfig,
}

#[derive(Debug, Serialize)]
pub struct OpenAiInputItem {
    pub role: String,
    pub content: Vec<OpenAiContentItem>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum OpenAiContentItem {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

#[derive(Debug, Serialize)]
pub struct OpenAiTextConfig {
    pub format: OpenAiJsonSchemaFormat,
}

#[derive(Debug, Serialize)]
pub struct OpenAiJsonSchemaFormat {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub strict: bool,
    pub schema: serde_json::Value,
}
