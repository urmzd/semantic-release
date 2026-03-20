pub mod claude;
pub mod copilot;
pub mod gemini;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub json_schema: Option<String>,
    pub working_dir: String,
}

#[derive(Debug, Clone)]
pub struct AiUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub text: String,
    pub usage: Option<AiUsage>,
}

/// Real-time events emitted during an AI request.
#[derive(Debug, Clone)]
pub enum AiEvent {
    ToolCall { tool: String, input: String },
}

#[async_trait]
pub trait AiBackend: Send + Sync {
    fn name(&self) -> &str;
    async fn is_available(&self) -> bool;
    async fn request(
        &self,
        req: &AiRequest,
        events: Option<mpsc::UnboundedSender<AiEvent>>,
    ) -> Result<AiResponse>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Backend {
    Claude,
    Copilot,
    Gemini,
}

pub struct BackendConfig {
    pub backend: Option<Backend>,
    pub model: Option<String>,
    pub budget: f64,
    pub debug: bool,
}

pub async fn resolve_backend(config: &BackendConfig) -> Result<Box<dyn AiBackend>> {
    let preferred = config.backend;

    let claude = claude::ClaudeBackend::new(config.model.clone(), config.budget, config.debug);
    let copilot = copilot::CopilotBackend::new(config.model.clone(), config.debug);
    let gemini = gemini::GeminiBackend::new(config.model.clone(), config.debug);

    // Helper: try all backends in order, returning the first available one
    let try_fallbacks = |backends: Vec<Box<dyn AiBackend>>| async move {
        for backend in backends {
            if backend.is_available().await {
                return Ok(backend);
            }
        }
        anyhow::bail!(crate::error::SrAiError::NoBackendAvailable)
    };

    match preferred {
        Some(Backend::Claude) => {
            if claude.is_available().await {
                return Ok(Box::new(claude));
            }
            eprintln!("Warning: claude CLI not found, falling back...");
            try_fallbacks(vec![Box::new(copilot), Box::new(gemini)]).await
        }
        Some(Backend::Copilot) => {
            if copilot.is_available().await {
                return Ok(Box::new(copilot));
            }
            eprintln!("Warning: gh models not available, falling back...");
            try_fallbacks(vec![Box::new(claude), Box::new(gemini)]).await
        }
        Some(Backend::Gemini) => {
            if gemini.is_available().await {
                return Ok(Box::new(gemini));
            }
            eprintln!("Warning: gemini CLI not found, falling back...");
            try_fallbacks(vec![Box::new(claude), Box::new(copilot)]).await
        }
        None => try_fallbacks(vec![Box::new(claude), Box::new(copilot), Box::new(gemini)]).await,
    }
}
