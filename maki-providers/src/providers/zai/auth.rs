use crate::AgentError;
use crate::providers::prompt_api_key;

const API_KEYS_URL: &str = "https://z.ai/manage-apikey/apikey-list";

pub fn login() -> Result<String, AgentError> {
    prompt_api_key(API_KEYS_URL, "ZHIPU_API_KEY")
}
