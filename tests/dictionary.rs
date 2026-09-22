use httpmock::prelude::*;
use translate_cli::dictionary::{self, Entry};

fn syn_body() -> serde_json::Value {
    serde_json::json!([
        {"word": "dead", "tags": ["adj", "n", "adv", "v"]},
        {"word": "abruptly", "tags": ["adv"]},
        {"word": "all of a sudden", "tags": ["adv"]},
        {"word": "mansion", "tags": ["n"]}
    ])
}

#[test]
fn groups_synonyms_under_the_head_words_part_of_speech() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/words")
            .query_param("sp", "suddenly");
        then.status(200)
            .json_body_obj(&serde_json::json!([{"word": "suddenly", "tags": ["adv"]}]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/words")
            .query_param("rel_syn", "suddenly");
        then.status(200).json_body_obj(&syn_body());
    });

    let got = dictionary::lookup_at(&server.base_url(), "suddenly").unwrap();

    assert_eq!(
        got,
        Entry {
            part_of_speech: "adverb".into(),
            words: vec![
                "suddenly".into(),
                "dead".into(),
                "abruptly".into(),
                "all of a sudden".into()
            ],
        }
    );
}

#[test]
fn multi_word_translations_are_not_looked_up() {
    let server = MockServer::start();
    let any = server.mock(|when, then| {
        when.any_request();
        then.status(200).json_body_obj(&serde_json::json!([]));
    });

    assert_eq!(
        dictionary::lookup_at(&server.base_url(), "good morning"),
        None
    );
    any.assert_calls(0);
}

#[test]
fn no_synonyms_means_no_entry() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/words").query_param("sp", "zzz");
        then.status(200)
            .json_body_obj(&serde_json::json!([{"word": "zzz", "tags": ["n"]}]));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/words")
            .query_param("rel_syn", "zzz");
        then.status(200).json_body_obj(&serde_json::json!([]));
    });

    assert_eq!(dictionary::lookup_at(&server.base_url(), "zzz"), None);
}

#[test]
fn api_failure_yields_no_entry() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.any_request();
        then.status(500).body("nope");
    });

    assert_eq!(dictionary::lookup_at(&server.base_url(), "suddenly"), None);
}
