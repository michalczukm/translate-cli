use httpmock::prelude::*;
use translate_cli::deepl::{Client, Error};

#[test]
fn translate_sends_expected_body_and_parses_response() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/v2/translate")
            .header("Authorization", "DeepL-Auth-Key k:fx")
            .json_body_includes(
                r#"{"text":["dzień dobry"],"target_lang":"EN-US","source_lang":"PL"}"#,
            );
        then.status(200).json_body_obj(&serde_json::json!({
            "translations": [{"text": "good morning", "detected_source_language": "PL"}]
        }));
    });

    let client = Client::with_base("k:fx".into(), server.base_url());
    let got = client
        .translate("dzień dobry", Some("PL"), "EN-US", None)
        .unwrap();

    m.assert();
    assert_eq!(got.text, "good morning");
    assert_eq!(got.detected_source_language, "PL");
}

#[test]
fn rephrase_returns_first_improvement() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(200).json_body_obj(&serde_json::json!({
            "improvements": [{"text": "Good morning! How are you?"}]
        }));
    });

    let client = Client::with_base("k:fx".into(), server.base_url());
    assert_eq!(
        client.rephrase("good morning", "EN-US").unwrap(),
        "Good morning! How are you?"
    );
}

#[test]
fn status_codes_map_to_errors() {
    for (status, expected) in [
        (403, Error::Auth),
        (429, Error::RateLimited),
        (456, Error::QuotaExceeded),
    ] {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v2/translate");
            then.status(status)
                .json_body_obj(&serde_json::json!({"message": "nope"}));
        });
        let client = Client::with_base("k:fx".into(), server.base_url());
        assert_eq!(
            client.translate("x", None, "PL", None).unwrap_err(),
            expected
        );
    }
}

#[test]
fn other_errors_surface_api_message() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v2/write/rephrase");
        then.status(400)
            .json_body_obj(&serde_json::json!({"message": "Value for target_lang not supported."}));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    assert_eq!(
        client.rephrase("x", "PL").unwrap_err(),
        Error::Api("Value for target_lang not supported.".into())
    );
}

#[test]
fn usage_is_parsed() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/v2/usage");
        then.status(200).json_body_obj(&serde_json::json!({
            "character_count": 50, "character_limit": 1000000
        }));
    });
    let client = Client::with_base("k:fx".into(), server.base_url());
    let u = client.usage().unwrap();
    assert_eq!((u.character_count, u.character_limit), (50, 1_000_000));
}
