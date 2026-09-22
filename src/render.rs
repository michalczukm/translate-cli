use crate::dictionary::Entry;
use crate::languages::display_name;

pub struct Output {
    pub source_text: String,
    pub did_you_mean: Option<String>,
    pub translation: String,
    pub alternative: Option<String>,
    pub definitions: Option<Entry>,
    pub source_lang: String,
    pub target_lang: String,
}

pub fn full(o: &Output, color: bool) -> String {
    let translation = if color {
        format!("\u{1b}[1m{}\u{1b}[0m", o.translation)
    } else {
        o.translation.clone()
    };
    let did_you_mean = match &o.did_you_mean {
        Some(fixed) => format!("Did you mean: {fixed}\n"),
        None => String::new(),
    };

    let mut s = format!("{}\n{}\n{}\n\n", o.source_text, did_you_mean, translation);

    match &o.definitions {
        Some(entry) => {
            s.push_str(&format!(
                "Definitions of {}\n[ {} -> {} ]\n\n{}\n",
                o.source_text,
                display_name(&o.source_lang),
                display_name(&o.target_lang),
                entry.part_of_speech,
            ));
            for word in &entry.words {
                s.push_str(&format!("    {word}\n"));
            }
            s.push_str(&format!("\n{}\n    {}\n", o.source_text, o.translation));
        }
        None => {
            s.push_str(&format!(
                "Translations of {}\n[ {} -> {} ]\n",
                o.source_text,
                display_name(&o.source_lang),
                display_name(&o.target_lang),
            ));
            if let Some(alt) = &o.alternative {
                s.push_str(&format!("\n    {alt}\n"));
            }
        }
    }
    s
}

pub fn plain(o: &Output) -> String {
    match &o.alternative {
        Some(alt) => format!("{}\n{}\n", o.translation, alt),
        None => format!("{}\n", o.translation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Output {
        Output {
            source_text: "dzień dobry, co słychać?".into(),
            did_you_mean: None,
            definitions: None,
            translation: "good morning, how are you?".into(),
            alternative: Some("Good morning! How are you?".into()),
            source_lang: "PL".into(),
            target_lang: "EN-US".into(),
        }
    }

    #[test]
    fn full_output_matches_trans_shape() {
        let expected = "dzień dobry, co słychać?\n\ngood morning, how are you?\n\nTranslations of dzień dobry, co słychać?\n[ Polski -> English ]\n\n    Good morning! How are you?\n";
        assert_eq!(full(&sample(), false), expected);
    }

    #[test]
    fn color_wraps_only_the_translation() {
        let out = full(&sample(), true);
        assert!(out.contains("\u{1b}[1mgood morning, how are you?\u{1b}[0m"));
        assert!(!out.contains("\u{1b}[1mdzień"));
    }

    #[test]
    fn full_output_without_alternative_ends_after_the_header() {
        let mut o = sample();
        o.alternative = None;
        let expected = "dzień dobry, co słychać?\n\ngood morning, how are you?\n\nTranslations of dzień dobry, co słychać?\n[ Polski -> English ]\n";
        assert_eq!(full(&o, false), expected);
    }

    #[test]
    fn did_you_mean_sits_under_the_source_line() {
        let mut o = sample();
        o.source_text = "gud mroning".into();
        o.did_you_mean = Some("Good morning".into());
        let expected = "gud mroning\nDid you mean: Good morning\n\ngood morning, how are you?\n\nTranslations of gud mroning\n[ Polski -> English ]\n\n    Good morning! How are you?\n";
        assert_eq!(full(&o, false), expected);
    }

    #[test]
    fn a_dictionary_entry_replaces_the_alternative_block() {
        let o = Output {
            source_text: "nagle".into(),
            did_you_mean: None,
            translation: "suddenly".into(),
            alternative: Some("ignored".into()),
            definitions: Some(Entry {
                part_of_speech: "adverb".into(),
                words: vec!["suddenly".into(), "abruptly".into()],
            }),
            source_lang: "PL".into(),
            target_lang: "EN-US".into(),
        };
        let expected = "nagle\n\nsuddenly\n\nDefinitions of nagle\n[ Polski -> English ]\n\nadverb\n    suddenly\n    abruptly\n\nnagle\n    suddenly\n";
        assert_eq!(full(&o, false), expected);
    }

    #[test]
    fn plain_output_is_two_lines() {
        assert_eq!(
            plain(&sample()),
            "good morning, how are you?\nGood morning! How are you?\n"
        );
    }

    #[test]
    fn plain_output_without_alternative_is_one_line() {
        let mut o = sample();
        o.alternative = None;
        assert_eq!(plain(&o), "good morning, how are you?\n");
    }
}
