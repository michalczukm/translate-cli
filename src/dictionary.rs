use serde::Deserialize;

const DATAMUSE: &str = "https://api.datamuse.com";

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub part_of_speech: String,
    pub words: Vec<String>,
}

#[derive(Deserialize)]
struct Word {
    word: String,
    #[serde(default)]
    tags: Vec<String>,
}

fn part_of_speech(tag: &str) -> Option<&'static str> {
    match tag {
        "n" => Some("noun"),
        "v" => Some("verb"),
        "adj" => Some("adjective"),
        "adv" => Some("adverb"),
        _ => None,
    }
}

pub fn lookup(word: &str) -> Option<Entry> {
    lookup_at(DATAMUSE, word)
}

/// Datamuse is English-only and single-word; callers gate on the target language.
pub fn lookup_at(base: &str, word: &str) -> Option<Entry> {
    let word = word.trim();
    if word.is_empty() || word.split_whitespace().count() > 1 {
        return None;
    }

    let head = fetch(base, "sp", word)?;
    let tag = head
        .first()
        .filter(|w| w.word.eq_ignore_ascii_case(word))?
        .tags
        .iter()
        .find_map(|t| part_of_speech(t))?;

    let synonyms: Vec<String> = fetch(base, "rel_syn", word)?
        .into_iter()
        .filter(|w| w.tags.iter().any(|t| part_of_speech(t) == Some(tag)))
        .map(|w| w.word)
        .collect();

    if synonyms.is_empty() {
        return None;
    }

    let mut words = vec![word.to_string()];
    words.extend(synonyms);
    Some(Entry {
        part_of_speech: tag.to_string(),
        words,
    })
}

fn fetch(base: &str, param: &str, word: &str) -> Option<Vec<Word>> {
    ureq::get(format!("{}/words", base.trim_end_matches('/')))
        .query(param, word)
        .query("md", "p")
        .query("max", "12")
        .call()
        .ok()?
        .body_mut()
        .read_json::<Vec<Word>>()
        .ok()
}
