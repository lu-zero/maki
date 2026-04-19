use crate::AgentError;
use crate::providers::prompt_api_key;

const API_KEYS_URL: &str = "https://console.mistral.ai/codestral/cli?profile_dialog=api-keys";
const CODING_PLAN_URL: &str = "https://console.mistral.ai/codestral/cli";

pub fn login() -> Result<String, AgentError> {
    prompt_api_key(API_KEYS_URL, "MISTRAL_API_KEY")
}

pub fn login_coding() -> Result<String, AgentError> {
    prompt_api_key(CODING_PLAN_URL, "MISTRAL_API_KEY")
}
