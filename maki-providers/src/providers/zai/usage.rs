use std::time::Duration;

use isahc::ReadResponseExt;
use isahc::config::Configurable;
use serde_json::Value;

use crate::AgentError;

const USAGE_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";
const API_KEY_ENV: &str = "ZHIPU_API_KEY";

pub fn fetch_usage() -> Result<String, AgentError> {
    let api_key = std::env::var(API_KEY_ENV).map_err(|_| AgentError::Config {
        message: format!("{API_KEY_ENV} not set"),
    })?;

    let client = isahc::HttpClient::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AgentError::Config {
            message: format!("http client: {e}"),
        })?;

    let request = isahc::Request::builder()
        .uri(USAGE_URL)
        .header("authorization", format!("Bearer {api_key}"))
        .body(())
        .map_err(|e| AgentError::Config {
            message: format!("build request: {e}"),
        })?;

    let mut response = client.send(request).map_err(|e| AgentError::Config {
        message: format!("usage request: {e}"),
    })?;

    let status = response.status().as_u16();
    if status != 200 {
        let body = response.text().unwrap_or_default();
        return Err(AgentError::Api {
            status,
            message: body,
        });
    }

    let body: Value = serde_json::from_str(&response.text().map_err(|e| AgentError::Config {
        message: format!("read body: {e}"),
    })?)
    .map_err(|e| AgentError::Config {
        message: format!("parse json: {e}"),
    })?;

    if !body["success"].as_bool().unwrap_or(false) {
        let msg = body["msg"].as_str().unwrap_or("unknown error");
        return Err(AgentError::Config {
            message: msg.into(),
        });
    }

    Ok(format_usage(&body))
}

fn format_usage(body: &Value) -> String {
    let data = &body["data"];
    let level = data["level"].as_str().unwrap_or("unknown");

    let mut lines = vec![format!("Plan: {level}")];

    for limit in data["limits"].as_array().into_iter().flatten() {
        let limit_type = limit["type"].as_str().unwrap_or("unknown");
        let pct = limit["percentage"].as_u64().unwrap_or(0);
        let reset = format_reset_time(limit["nextResetTime"].as_u64().unwrap_or(0));

        match limit_type {
            "TOKENS_LIMIT" => {
                lines.push(format!("  Tokens: {pct}% used, resets {reset}"));
            }
            "TIME_LIMIT" => {
                let usage = limit["usage"].as_u64().unwrap_or(0);
                let remaining = limit["remaining"].as_u64().unwrap_or(0);
                lines.push(format!(
                    "  Time: {remaining}/{usage} remaining ({pct}% used), resets {reset}"
                ));
            }
            _ => {
                lines.push(format!("  {limit_type}: {pct}% used, resets {reset}"));
            }
        }
    }

    lines.join("\n")
}

fn format_reset_time(ts_ms: u64) -> String {
    let secs = i64::try_from(ts_ms / 1000).unwrap_or(0);
    let ts = jiff::Timestamp::from_second(secs).unwrap_or(jiff::Timestamp::UNIX_EPOCH);
    let zoned = ts.to_zoned(jiff::tz::TimeZone::system());
    zoned.strftime("%Y-%m-%d %H:%M").to_string()
}
