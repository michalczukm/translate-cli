use httpmock::prelude::*;
use translate_cli::{deepl::Client, variants};

#[test]
fn rephrase_tier_is_used_for_write_supported_targets() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(200).json_body_obj(&serde_json::json!({
            "improvements": [{"text": "Good morning! How are you?"}]
        }));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    let got = variants::alternative(&client, "dzień dobry", Some("PL"), "EN-US", "good morning");
    m.assert();
    assert_eq!(got, Some("Good morning! How are you?".into()));
}

#[test]
fn formality_tier_retranslates_the_source_for_pl() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/v2/translate").body_includes("prefer_more");
        then.status(200).json_body_obj(&serde_json::json!({
            "translations": [{"text": "Dzień dobry, jak się Państwo mają?", "detected_source_language": "EN"}]
        }));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    let got = variants::alternative(
        &client,
        "good morning",
        Some("EN"),
        "PL",
        "Dzień dobry, jak się masz?",
    );
    m.assert();
    assert_eq!(got, Some("Dzień dobry, jak się Państwo mają?".into()));
}

#[test]
fn identical_alternative_is_dropped() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(200)
            .json_body_obj(&serde_json::json!({"improvements": [{"text": "good morning"}]}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    assert_eq!(
        variants::alternative(&client, "x", None, "EN-US", "good morning"),
        None
    );
}

#[test]
fn api_failure_yields_none_not_an_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(500)
            .json_body_obj(&serde_json::json!({"message": "boom"}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    assert_eq!(
        variants::alternative(&client, "x", None, "EN-US", "good morning"),
        None
    );
}

#[test]
fn unsupported_target_yields_none_without_calling_out() {
    let server = MockServer::start();
    let any = server.mock(|when, then| {
        when.any_request();
        then.status(200).json_body_obj(&serde_json::json!({}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    assert_eq!(
        variants::alternative(&client, "x", None, "UK", "привіт"),
        None
    );
    any.assert_calls(0);
}

#[test]
fn corrects_typos_when_the_source_supports_write() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(200).json_body_obj(&serde_json::json!({
            "improvements": [{"text": "Good morning! How are you?"}]
        }));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());

    let got = variants::fix_source(&client, "gud mroning, how ar yu?", Some("EN"));

    m.assert();
    assert_eq!(got, Some("Good morning! How are you?".into()));
}

#[test]
fn unchanged_text_yields_no_correction() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(200)
            .json_body_obj(&serde_json::json!({"improvements": [{"text": "good morning"}]}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());

    assert_eq!(
        variants::fix_source(&client, "good morning", Some("EN")),
        None
    );
}

#[test]
fn polish_source_is_never_called_out() {
    let server = MockServer::start();
    let any = server.mock(|when, then| {
        when.any_request();
        then.status(200).json_body_obj(&serde_json::json!({}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());

    assert_eq!(
        variants::fix_source(&client, "dzien dobri", Some("PL")),
        None
    );
    any.assert_calls(0);
}

#[test]
fn auto_detected_source_is_never_called_out() {
    let server = MockServer::start();
    let any = server.mock(|when, then| {
        when.any_request();
        then.status(200).json_body_obj(&serde_json::json!({}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());

    assert_eq!(variants::fix_source(&client, "gud mroning", None), None);
    any.assert_calls(0);
}

#[test]
fn api_failure_yields_no_correction() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(500)
            .json_body_obj(&serde_json::json!({"message": "boom"}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());

    assert_eq!(
        variants::fix_source(&client, "gud mroning", Some("EN")),
        None
    );
}
