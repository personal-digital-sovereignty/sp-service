use serde::{Deserialize, Serialize};
use serde_json::Value;

// ==========================================
// Agentic Workflows: Plan & Execute Models
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteStep {
    pub task: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteBlueprint {
    pub plan: Vec<PlanExecuteStep>,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

// ==========================================
// Tools & Agentic Capabilities
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON schema object
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    #[serde(default)] // Permite falha silenciosa para modelos puros (se ausente)
    pub model: String,

    // O Vercel AI SDK muitas vezes manda o prompt num campo "input" em vez de "messages" (Endpoint /responses)
    #[serde(alias = "input")]
    pub messages: Vec<OpenAIChatMessage>,

    // Tools Injection Opcional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,

    // Extensão O.S (Cybrid Router) para isolamento de Contexto
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,

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

    // Extensão Cíbrida p/ Persistência Local
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

// ==========================================
// OpenAI-Compatible Response Models (Blocking)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OpenAIChatChoiceMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OpenAIChatChoice {
    pub index: i32,
    pub message: OpenAIChatChoiceMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct OpenAITokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChunkToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkFunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ChunkFunctionCall>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAITokenUsage>,
}