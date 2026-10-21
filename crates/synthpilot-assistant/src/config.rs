use std::fmt;

use thiserror::Error;

pub(crate) const PROVIDER: &str = "SYNTHPILOT_AI_PROVIDER";
pub(crate) const MODEL: &str = "SYNTHPILOT_AI_MODEL";
pub(crate) const API_KEY: &str = "SYNTHPILOT_AI_API_KEY";
pub(crate) const BASE_URL: &str = "SYNTHPILOT_AI_BASE_URL";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    OpenAi,
    Anthropic,
}

impl Provider {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "openai" | "chatgpt" => Some(Self::OpenAi),
            "anthropic" | "claude" => Some(Self::Anthropic),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
        }
    }

    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("{PROVIDER} is \"{0}\". Use openai or anthropic.")]
    UnknownProvider(String),
    #[error("Set {0} in your .env file to use the assistant.")]
    Missing(&'static str),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    pub provider: Provider,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("api_key", &"<hidden>")
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Option<Self>, ConfigError> {
        let _ = dotenvy::dotenv();
        Self::from_vars(|name| std::env::var(name).ok())
    }

    pub fn from_vars(read: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, ConfigError> {
        let value = |name: &str| {
            read(name)
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let provider = value(PROVIDER);
        let model = value(MODEL);
        let api_key = value(API_KEY);
        let base_url = value(BASE_URL);
        if provider.is_none() && model.is_none() && api_key.is_none() {
            return Ok(None);
        }
        let provider_text = provider.ok_or(ConfigError::Missing(PROVIDER))?;
        let provider =
            Provider::parse(&provider_text).ok_or(ConfigError::UnknownProvider(provider_text))?;
        let model = model.ok_or(ConfigError::Missing(MODEL))?;
        let api_key = match api_key {
            Some(key) => key,
            None if base_url.is_some() => String::new(),
            None => return Err(ConfigError::Missing(API_KEY)),
        };
        let base_url = base_url
            .unwrap_or_else(|| provider.default_base_url().to_owned())
            .trim_end_matches('/')
            .to_owned();
        Ok(Some(Self {
            provider,
            model,
            api_key,
            base_url,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn read(pairs: &[(&str, &str)]) -> Result<Option<Config>, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        Config::from_vars(|name| map.get(name).cloned())
    }

    #[test]
    fn nothing_set_means_no_assistant() {
        assert_eq!(read(&[]), Ok(None));
        assert_eq!(read(&[(PROVIDER, " "), (MODEL, "")]), Ok(None));
    }

    #[test]
    fn reads_openai_and_anthropic() {
        let openai = read(&[(PROVIDER, "openai"), (MODEL, "gpt"), (API_KEY, " sk ")])
            .unwrap()
            .unwrap();
        assert_eq!(openai.provider, Provider::OpenAi);
        assert_eq!(openai.api_key, "sk");
        assert_eq!(openai.base_url, "https://api.openai.com/v1");

        let anthropic = read(&[(PROVIDER, "Claude"), (MODEL, "m"), (API_KEY, "k")])
            .unwrap()
            .unwrap();
        assert_eq!(anthropic.provider, Provider::Anthropic);
        assert_eq!(anthropic.base_url, "https://api.anthropic.com/v1");
        assert_eq!(Provider::Anthropic.name(), "Anthropic");
        assert_eq!(Provider::OpenAi.name(), "OpenAI");
    }

    #[test]
    fn local_servers_need_no_key() {
        let local = read(&[
            (PROVIDER, "openai"),
            (MODEL, "llama"),
            (BASE_URL, "http://localhost:11434/v1/"),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(local.base_url, "http://localhost:11434/v1");
        assert_eq!(local.api_key, "");
    }

    #[test]
    fn explains_what_is_missing() {
        assert_eq!(read(&[(MODEL, "gpt")]), Err(ConfigError::Missing(PROVIDER)));
        assert_eq!(
            read(&[(PROVIDER, "openai"), (API_KEY, "k")]),
            Err(ConfigError::Missing(MODEL))
        );
        assert_eq!(
            read(&[(PROVIDER, "openai"), (MODEL, "gpt")]),
            Err(ConfigError::Missing(API_KEY))
        );
        let unknown = read(&[(PROVIDER, "bard"), (MODEL, "x"), (API_KEY, "k")]);
        assert_eq!(
            unknown,
            Err(ConfigError::UnknownProvider("bard".to_owned()))
        );
        assert_eq!(
            unknown.unwrap_err().to_string(),
            "SYNTHPILOT_AI_PROVIDER is \"bard\". Use openai or anthropic."
        );
    }

    #[test]
    fn debug_output_hides_the_key() {
        let config = read(&[(PROVIDER, "openai"), (MODEL, "gpt"), (API_KEY, "secret")])
            .unwrap()
            .unwrap();
        let text = format!("{config:?}");
        assert!(!text.contains("secret"));
        assert!(text.contains("<hidden>"));
    }
}
