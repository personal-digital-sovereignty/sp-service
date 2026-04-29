use reqwest::Client;
use crate::models::{OpenAIChatRequest, OpenRouterSettings, OpenAIChatResponse};
use anyhow::{Result, anyhow};
use reqwest_eventsource::EventSource;
use tracing::error;

pub struct OpenRouterClient {
    client: Client,
    settings: OpenRouterSettings,
}

impl OpenRouterClient {
    pub fn new(settings: OpenRouterSettings) -> Self {
        Self {
            client: Client::new(),
            settings,
        }
    }

    /// Execução de chat completions (Blocking)
    #[allow(dead_code)]
    pub async fn chat_completions(&self, request: OpenAIChatRequest) -> Result<OpenAIChatResponse> {
        let url = format!("{}/chat/completions", self.settings.base_url.trim_end_matches('/'));
        
        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.settings.api_key))
            .header("HTTP-Referer", &self.settings.site_url)
            .header("X-Title", &self.settings.site_name)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            error!("OpenRouter Error: {}", err_text);
            return Err(anyhow!("OpenRouter API error: {}", err_text));
        }

        let res_json = response.json::<OpenAIChatResponse>().await?;
        Ok(res_json)
    }

    /// Execução de chat completions com SSE Stream
    pub async fn chat_completions_stream(&self, mut request: OpenAIChatRequest) -> Result<EventSource> {
        let url = format!("{}/chat/completions", self.settings.base_url.trim_end_matches('/'));
        request.stream = Some(true);

        let event_source = EventSource::new(
            self.client.post(&url)
                .header("Authorization", format!("Bearer {}", self.settings.api_key))
                .header("HTTP-Referer", &self.settings.site_url)
                .header("X-Title", &self.settings.site_name)
                .json(&request)
        ).map_err(|e| anyhow!("Failed to create EventSource: {}", e))?;

        Ok(event_source)
    }
}
