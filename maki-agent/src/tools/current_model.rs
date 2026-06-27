//! A simple tool that returns information about the current model being used.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ToolContext;
use crate::tools::schema::{ParamSchema, ToolInputError};
use crate::ToolOutput;

/// Tool that returns the current model information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CurrentModel;

impl CurrentModel {
    pub const NAME: &str = "current_model";
    pub const DESCRIPTION: &str = "Returns information about the current model being used. No parameters.";
    pub const EXAMPLES: Option<&str> = Some(r#"[{}]"#);

    pub(crate) const SCHEMA: &'static crate::tools::schema::ParamSchema =
        &ParamSchema::Object {
            properties: &[],
            description: "No parameters required",
        };

    pub(crate) fn parse_input(input: &Value) -> Result<Self, ToolInputError> {
        use crate::tools::schema::{ParamKind, ToolInputErrorKind, JsonPath};

        let sanitized = crate::tools::sanitize_tool_input(input);
        match sanitized {
            Value::Object(obj) if obj.is_empty() => Ok(CurrentModel),
            Value::Object(_) => Err(ToolInputError {
                path: JsonPath::default(),
                kind: ToolInputErrorKind::Missing { expected: "no parameters" },
            }),
            _ => Err(ToolInputError {
                path: JsonPath::default(),
                kind: ToolInputErrorKind::TypeMismatch {
                    expected: ParamKind::Object,
                    got: ParamKind::of(&sanitized),
                    preview: None,
                },
            }),
        }
    }

    pub async fn execute(&self, ctx: &ToolContext) -> Result<ToolOutput, String> {
        let model = &ctx.model;
        let info = format!(
            "Current model: {} (provider: {}, tier: {}, family: {:?})",
            model.spec(),
            model.provider,
            model.tier,
            model.family
        );
        Ok(ToolOutput::Plain(info.into()))
    }
}

super::impl_tool!(CurrentModel, audience = super::ToolAudience::MAIN);

impl super::ToolInvocation for CurrentModel {
    fn start_header(&self) -> super::HeaderFuture {
        super::HeaderFuture::Ready(super::HeaderResult::plain(
            "Current model information".to_string(),
        ))
    }

    fn execute<'a>(
        self: Box<Self>,
        ctx: &'a super::ToolContext,
    ) -> super::ExecFuture<'a> {
        Box::pin(async move { CurrentModel::execute(&self, ctx).await.into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::test_support::stub_ctx;
    use maki_providers::model::{Model, ModelFamily, ModelTier};
    use maki_providers::provider::ProviderKind;
    use std::sync::Arc;

    #[test]
    fn current_model_returns_model_info() {
        let model = Arc::new(Model {
            id: "claude-sonnet-4-20250514".to_string(),
            provider: ProviderKind::Anthropic,
            dynamic_slug: None,
            tier: ModelTier::Medium,
            family: ModelFamily::Claude,
            supports_tool_examples_override: None,
            pricing: maki_providers::model::ModelPricing::ZERO,
            max_output_tokens: 4096,
            context_window: 200000,
        });

        let mut ctx = stub_ctx(&crate::AgentMode::Build);
        ctx.model = Arc::clone(&model);

        let tool = CurrentModel;
        let result = smol::block_on(async { tool.execute(&ctx).await });

        match result {
            Ok(ToolOutput::Plain(text)) => {
                let text_str = text.text.as_str();
                assert!(text_str.contains("claude-sonnet-4-20250514"));
                assert!(text_str.contains("anthropic"));
                assert!(text_str.contains("medium"));
                assert!(text_str.contains("Claude"));
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[test]
    fn current_model_parses_empty_input() {
        use serde_json::json;

        let input = json!({});
        let result = CurrentModel::parse_input(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn current_model_rejects_non_empty_input() {
        use serde_json::json;

        let input = json!({"unknown_field": 42});
        let result = CurrentModel::parse_input(&input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("internal validator bug"));
        assert!(msg.contains("required") || msg.contains("expected"));
    }

    #[test]
    fn current_model_rejects_string_input() {
        use serde_json::json;

        let input = json!("raw string");
        let result = CurrentModel::parse_input(&input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("internal validator bug"));
    }
}
