use crate::deepl::Client;
use crate::languages::{supports_formality, supports_rephrase};

/// Typos derail DeepL's language detection, so a correction pass is only safe
/// when the source language was stated explicitly.
pub fn fix_source(client: &Client, text: &str, source_lang: Option<&str>) -> Option<String> {
    let source = source_lang?;
    if !supports_rephrase(source) {
        return None;
    }

    let corrected = client.rephrase(text, source).ok()?;
    (corrected.trim() != text.trim()).then_some(corrected)
}

/// DeepL Write covers only some targets (not PL), so formality-capable targets
/// get their alternative from a second translation instead.
pub fn alternative(
    client: &Client,
    source_text: &str,
    source_lang: Option<&str>,
    target: &str,
    translated: &str,
) -> Option<String> {
    let candidate = if supports_rephrase(target) {
        client.rephrase(translated, target).ok()
    } else if supports_formality(target) {
        client
            .translate(source_text, source_lang, target, Some("prefer_more"))
            .ok()
            .map(|t| t.text)
    } else {
        None
    }?;

    (candidate.trim() != translated.trim()).then_some(candidate)
}
