use serde::{Deserialize, Serialize};
use serde_json::Value;

// ==========================================
// Sovereign Ecosystem Log Models
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String, // info, warn, error, agent, rag
    pub message: String,
}

// ==========================================
// OpenAI-Compatible Request Models
// ==========================================

/// O conteúdo de uma mensagem de chat pode ser uma string pura ou um array de objetos (multimodal/image)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Multimodal(Vec<Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    #[serde(default)] // Permite falha silenciosa para modelos puros (se ausente)
    pub model: String,
    
    // O Vercel AI SDK muitas vezes manda o prompt num campo "input" em vez de "messages" (Endpoint /responses)
    #[serde(alias = "input")]
    pub messages: Vec<OpenAIChatMessage>,
    
    // Parâmetros de Inferência Opcionais
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    
    // Suporte genérico para Stop (pode ser String ou Vec<String>)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

// ==========================================
// OpenAI-Compatible Response Models (Blocking)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatChoiceMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatChoice {
    pub index: i32,
    pub message: OpenAIChatChoiceMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAITokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAIChatChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAITokenUsage>,
}

// ==========================================
// OpenAI-Compatible SSE Stream Models (Chunking)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatChunkChoice {
    pub index: i32,
    pub delta: OpenAIChatChunkDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatChunkResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAIChatChunkChoice>,
}
