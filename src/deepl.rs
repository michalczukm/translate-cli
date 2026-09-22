use serde::Deserialize;
use serde_json::{Value, json};

use crate::languages::normalize_target;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Auth,
    RateLimited,
    QuotaExceeded,
    Api(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Auth => write!(f, "invalid DeepL API key"),
            Error::RateLimited => write!(f, "rate limited by DeepL, try again shortly"),
            Error::QuotaExceeded => write!(f, "DeepL character quota exhausted"),
            Error::Api(m) => write!(f, "{m}"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Translation {
    pub text: String,
    #[serde(default)]
    pub detected_source_language: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub character_count: u64,
    pub character_limit: u64,
}

#[derive(Deserialize)]
struct Translations {
    translations: Vec<Translation>,
}

#[derive(Deserialize)]
struct Improvement {
    text: String,
}

#[derive(Deserialize)]
struct Improvements {
    improvements: Vec<Improvement>,
}

fn base_for(key: &str) -> String {
    if key.trim_end().ends_with(":fx") {
        "https://api-free.deepl.com".to_string()
    } else {
        "https://api.deepl.com".to_string()
    }
}

fn read<T: serde::de::DeserializeOwned>(
    res: &mut ureq::http::Response<ureq::Body>,
) -> Result<T, Error> {
    let status = res.status().as_u16();
    let raw = res
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::Api(e.to_string()))?;

    if (200..300).contains(&status) {
        return serde_json::from_str(&raw).map_err(|e| Error::Api(e.to_string()));
    }

    let message = serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("HTTP {status}"));

    Err(match status {
        403 => Error::Auth,
        429 => Error::RateLimited,
        456 => Error::QuotaExceeded,
        _ => Error::Api(message),
    })
}

pub struct Client {
    key: String,
    base: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(key: String) -> Self {
        let base = base_for(&key);
        Self::with_base(key, base)
    }

    pub fn with_base(key: String, base: String) -> Self {
        // Status errors are handled inline so the API's own `message` stays readable.
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            key,
            base: base.trim_end_matches('/').to_string(),
            agent,
        }
    }

    fn post<T: serde::de::DeserializeOwned>(&self, path: &str, body: Value) -> Result<T, Error> {
        let mut res = self
            .agent
            .post(format!("{}{path}", self.base))
            .header("Authorization", format!("DeepL-Auth-Key {}", self.key))
            .send_json(&body)
            .map_err(|e| Error::Api(e.to_string()))?;
        read(&mut res)
    }

    pub fn translate(
        &self,
        text: &str,
        source: Option<&str>,
        target: &str,
        formality: Option<&str>,
    ) -> Result<Translation, Error> {
        let mut body = json!({ "text": [text], "target_lang": normalize_target(target) });
        if let Some(s) = source {
            body["source_lang"] = json!(s);
        }
        if let Some(f) = formality {
            body["formality"] = json!(f);
        }

        let parsed: Translations = self.post("/v2/translate", body)?;
        parsed
            .translations
            .into_iter()
            .next()
            .ok_or_else(|| Error::Api("DeepL returned no translation".into()))
    }

    pub fn rephrase(&self, text: &str, target: &str) -> Result<String, Error> {
        let body = json!({ "text": [text], "target_lang": normalize_target(target) });
        let parsed: Improvements = self.post("/v2/write/rephrase", body)?;
        parsed
            .improvements
            .into_iter()
            .next()
            .map(|i| i.text)
            .ok_or_else(|| Error::Api("DeepL returned no improvement".into()))
    }

    pub fn usage(&self) -> Result<Usage, Error> {
        let mut res = self
            .agent
            .get(format!("{}/v2/usage", self.base))
            .header("Authorization", format!("DeepL-Auth-Key {}", self.key))
            .call()
            .map_err(|e| Error::Api(e.to_string()))?;
        read(&mut res)
    }
}

#[cfg(test)]
mod tests {
    use super::base_for;

    #[test]
    fn free_keys_use_the_free_host() {
        assert_eq!(base_for("abc:fx"), "https://api-free.deepl.com");
        assert_eq!(base_for("abc"), "https://api.deepl.com");
    }
}
