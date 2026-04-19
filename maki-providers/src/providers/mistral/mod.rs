use std::sync::{Arc, Mutex};

use flume::Sender;
use serde_json::{Value, json};

use crate::model::{Model, ModelEntry, ModelFamily, ModelPricing, ModelTier};
use crate::provider::{BoxFuture, Provider};
use crate::{AgentError, Message, ProviderEvent, StreamResponse, ThinkingConfig};

use super::ResolvedAuth;
use super::openai_compat::{OpenAiCompatConfig, OpenAiCompatProvider};

pub mod auth;

#[derive(Debug, Clone, Copy)]
pub enum MistralPlan {
    Standard,
    Coding,
}

static CONFIG: OpenAiCompatConfig = OpenAiCompatConfig {
    api_key_env: "MISTRAL_API_KEY",
    base_url: "https://api.mistral.ai/v1",
    max_tokens_field: "max_tokens",
    include_stream_usage: true,
    provider_name: "Mistral",
};

const MISTRAL_MEDIUM_3_5: ModelEntry = ModelEntry {
    prefixes: &["mistral-vibe-cli-latest", "mistral-medium-3.5"],
    tier: ModelTier::Strong,
    family: ModelFamily::Generic,
    default: true,
    pricing: ModelPricing {
        input: 1.5,
        output: 7.5,
        cache_write: 0.0,
        cache_read: 0.0,
    },
    max_output_tokens: 262_144,
    context_window: 262_144,
};

const DEVSTRAL_SMALL: ModelEntry = ModelEntry {
    prefixes: &["devstral-small-latest", "devstral-small"],
    tier: ModelTier::Weak,
    family: ModelFamily::Generic,
    default: true,
    pricing: ModelPricing {
        input: 0.1,
        output: 0.3,
        cache_write: 0.0,
        cache_read: 0.0,
    },
    max_output_tokens: 262_144,
    context_window: 262_144,
};

const MISTRAL_MEDIUM: ModelEntry = ModelEntry {
    prefixes: &["mistral-medium-latest", "mistral-medium-2508"],
    tier: ModelTier::Medium,
    family: ModelFamily::Generic,
    default: true,
    pricing: ModelPricing {
        input: 0.4,
        output: 2.0,
        cache_write: 0.0,
        cache_read: 0.0,
    },
    max_output_tokens: 131_072,
    context_window: 131_072,
};

pub(crate) fn models() -> &'static [ModelEntry] {
    &[MISTRAL_MEDIUM_3_5, MISTRAL_MEDIUM, DEVSTRAL_SMALL]
}

pub(crate) fn models_coding() -> &'static [ModelEntry] {
    &[MISTRAL_MEDIUM_3_5, MISTRAL_MEDIUM, DEVSTRAL_SMALL]
}

pub struct Mistral {
    compat: OpenAiCompatProvider,
    auth: Arc<Mutex<ResolvedAuth>>,
    system_prefix: Option<String>,
}

impl Mistral {
    pub fn new(_plan: MistralPlan, timeouts: super::Timeouts) -> Result<Self, AgentError> {
        let api_key = std::env::var(CONFIG.api_key_env).map_err(|_| AgentError::Config {
            message: format!("{} not set", CONFIG.api_key_env),
        })?;
        Ok(Self {
            compat: OpenAiCompatProvider::new(&CONFIG, timeouts),
            auth: Arc::new(Mutex::new(ResolvedAuth::bearer(&api_key))),
            system_prefix: None,
        })
    }

    pub(crate) fn with_auth(auth: Arc<Mutex<ResolvedAuth>>, timeouts: super::Timeouts) -> Self {
        Self {
            compat: OpenAiCompatProvider::new(&CONFIG, timeouts),
            auth,
            system_prefix: None,
        }
    }

    pub(crate) fn with_system_prefix(mut self, prefix: Option<String>) -> Self {
        self.system_prefix = prefix;
        self
    }
}

impl Provider for Mistral {
    fn stream_message<'a>(
        &'a self,
        model: &'a Model,
        messages: &'a [Message],
        system: &'a str,
        tools: &'a Value,
        event_tx: &'a Sender<ProviderEvent>,
        thinking: ThinkingConfig,
        session_id: Option<&'a str>,
    ) -> BoxFuture<'a, Result<StreamResponse, AgentError>> {
        Box::pin(async move {
            let auth = self.auth.lock().unwrap().clone();
            let mut buf = String::new();
            let system = super::with_prefix(&self.system_prefix, system, &mut buf);
            let mut body = self.compat.build_body(model, messages, system, tools);

            if !matches!(thinking, ThinkingConfig::Off) {
                body["reasoning_effort"] = json!("high");
            }

            let mut extra_headers = vec![];
            if let Some(session_id) = session_id {
                extra_headers.push(("x-affinity".to_string(), session_id.to_string()));
            }
            self.compat
                .do_stream(model, &extra_headers, &body, event_tx, &auth)
                .await
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<Vec<String>, AgentError>> {
        Box::pin(async move {
            let auth = self.auth.lock().unwrap().clone();
            self.compat.do_list_models(&auth).await
        })
    }
}
