use serde_json::{Value, json};

use super::assistant::AssistantError;
use super::config::{Config, Provider};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;

pub(crate) struct Request {
    pub(crate) url: String,
    pub(crate) headers: Vec<(&'static str, String)>,
    pub(crate) body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    User,
    Assistant,
}

impl Role {
    const fn name(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

fn messages(turns: &[(Role, String)]) -> Vec<Value> {
    turns
        .iter()
        .map(|(role, text)| json!({"role": role.name(), "content": text}))
        .collect()
}

pub(crate) fn request(config: &Config, system: &str, turns: &[(Role, String)]) -> Request {
    match config.provider {
        Provider::OpenAi => {
            let mut headers = vec![("content-type", "application/json".to_owned())];
            if !config.api_key.is_empty() {
                headers.push(("authorization", format!("Bearer {}", config.api_key)));
            }
            Request {
                url: format!("{}/chat/completions", config.base_url),
                headers,
                body: json!({
                    "model": config.model,
                    "messages": std::iter::once(json!({"role": "system", "content": system}))
                        .chain(messages(turns))
                        .collect::<Vec<_>>(),
                    "response_format": {"type": "json_object"},
                })
                .to_string(),
            }
        }
        Provider::Anthropic => Request {
            url: format!("{}/messages", config.base_url),
            headers: vec![
                ("content-type", "application/json".to_owned()),
                ("x-api-key", config.api_key.clone()),
                ("anthropic-version", ANTHROPIC_VERSION.to_owned()),
            ],
            body: json!({
                "model": config.model,
                "max_tokens": MAX_TOKENS,
                "system": system,
                "messages": messages(turns),
            })
            .to_string(),
        },
    }
}

pub(crate) fn reply(provider: Provider, status: u16, body: &str) -> Result<String, AssistantError> {
    let json: Option<Value> = serde_json::from_str(body).ok();
    if !(200..300).contains(&status) {
        let message = json
            .as_ref()
            .and_then(|json| json.pointer("/error/message"))
            .and_then(Value::as_str)
            .map_or_else(|| body.chars().take(300).collect(), ToOwned::to_owned);
        return Err(AssistantError::Service { status, message });
    }
    let json = json.ok_or_else(|| AssistantError::Format("the reply was not JSON".to_owned()))?;
    let text = match provider {
        Provider::OpenAi => json
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        Provider::Anthropic => json.get("content").and_then(Value::as_array).map(|blocks| {
            blocks
                .iter()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<String>()
        }),
    };
    text.filter(|text| !text.trim().is_empty())
        .ok_or_else(|| AssistantError::Format("the reply had no text".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(provider: Provider, key: &str) -> Config {
        Config {
            provider,
            model: "model-x".to_owned(),
            api_key: key.to_owned(),
            base_url: provider.default_base_url().to_owned(),
        }
    }

    #[test]
    fn builds_openai_requests() {
        let request = request(
            &config(Provider::OpenAi, "sk"),
            "system",
            &[(Role::User, "user".to_owned())],
        );
        assert_eq!(request.url, "https://api.openai.com/v1/chat/completions");
        assert!(
            request
                .headers
                .contains(&("authorization", "Bearer sk".to_owned()))
        );
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["model"], "model-x");
        assert_eq!(body["messages"][0]["content"], "system");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["response_format"]["type"], "json_object");

        let local = request_without_key();
        assert!(
            local
                .headers
                .iter()
                .all(|(name, _)| *name != "authorization")
        );
    }

    fn request_without_key() -> Request {
        request(
            &config(Provider::OpenAi, ""),
            "s",
            &[(Role::User, "u".to_owned())],
        )
    }

    #[test]
    fn builds_anthropic_requests() {
        let request = request(
            &config(Provider::Anthropic, "key"),
            "system",
            &[
                (Role::User, "user".to_owned()),
                (Role::Assistant, "lookup".to_owned()),
                (Role::User, "options".to_owned()),
            ],
        );
        assert_eq!(request.url, "https://api.anthropic.com/v1/messages");
        assert!(request.headers.contains(&("x-api-key", "key".to_owned())));
        assert!(
            request
                .headers
                .contains(&("anthropic-version", "2023-06-01".to_owned()))
        );
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["system"], "system");
        assert_eq!(body["messages"][0]["content"], "user");
        assert_eq!(body["messages"][1]["role"], "assistant");
        assert_eq!(body["messages"][2]["content"], "options");
        assert_eq!(body["max_tokens"], 4096);
    }

    #[test]
    fn reads_replies() {
        let openai = r#"{"choices":[{"message":{"content":"{\"summary\":\"ok\"}"}}]}"#;
        assert_eq!(
            reply(Provider::OpenAi, 200, openai),
            Ok("{\"summary\":\"ok\"}".to_owned())
        );
        let anthropic =
            r#"{"content":[{"type":"thinking","thinking":"x"},{"type":"text","text":"{}"}]}"#;
        assert_eq!(
            reply(Provider::Anthropic, 200, anthropic),
            Ok("{}".to_owned())
        );
    }

    #[test]
    fn reports_service_errors() {
        let error = r#"{"error":{"message":"Incorrect API key provided"}}"#;
        assert_eq!(
            reply(Provider::OpenAi, 401, error),
            Err(AssistantError::Service {
                status: 401,
                message: "Incorrect API key provided".to_owned()
            })
        );
        assert_eq!(
            reply(Provider::Anthropic, 502, "Bad gateway"),
            Err(AssistantError::Service {
                status: 502,
                message: "Bad gateway".to_owned()
            })
        );
        assert!(matches!(
            reply(Provider::OpenAi, 200, "<html>"),
            Err(AssistantError::Format(_))
        ));
        assert!(matches!(
            reply(Provider::Anthropic, 200, r#"{"content":[]}"#),
            Err(AssistantError::Format(_))
        ));
    }
}
