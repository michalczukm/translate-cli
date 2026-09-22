const REPHRASE: [&str; 8] = ["DE", "EN-GB", "EN-US", "ES", "FR", "IT", "PT-BR", "PT-PT"];
const FORMALITY: [&str; 10] = [
    "DE", "ES", "FR", "IT", "JA", "NL", "PL", "PT-BR", "PT-PT", "RU",
];
const NAMES: [(&str, &str); 2] = [("EN", "English"), ("PL", "Polski")];

/// Bare `EN` and `PT` are rejected by DeepL as targets; they need a region.
pub fn normalize_target(code: &str) -> String {
    let c = code.trim().to_uppercase();
    match c.as_str() {
        "EN" => "EN-US".to_string(),
        "PT" => "PT-PT".to_string(),
        _ => c,
    }
}

pub fn normalize_source(code: &str) -> String {
    let c = code.trim().to_uppercase();
    c.split('-').next().unwrap_or(&c).to_string()
}

pub fn display_name(code: &str) -> String {
    let base = normalize_source(code);
    NAMES
        .iter()
        .find(|(k, _)| *k == base)
        .map(|(_, v)| (*v).to_string())
        .unwrap_or(base)
}

pub fn supports_rephrase(target: &str) -> bool {
    REPHRASE.contains(&normalize_target(target).as_str())
}

pub fn supports_formality(target: &str) -> bool {
    FORMALITY.contains(&normalize_target(target).as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_en_normalizes_to_en_us() {
        assert_eq!(normalize_target("en"), "EN-US");
        assert_eq!(normalize_target("EN"), "EN-US");
        assert_eq!(normalize_target("en-gb"), "EN-GB");
        assert_eq!(normalize_target("pl"), "PL");
        assert_eq!(normalize_target("pt-br"), "PT-BR");
    }

    #[test]
    fn source_drops_region() {
        assert_eq!(normalize_source("en-us"), "EN");
        assert_eq!(normalize_source("PT-BR"), "PT");
        assert_eq!(normalize_source("pl"), "PL");
    }

    #[test]
    fn display_names_are_endonyms_with_code_fallback() {
        assert_eq!(display_name("PL"), "Polski");
        assert_eq!(display_name("EN-US"), "English");
        assert_eq!(display_name("XX"), "XX");
    }

    #[test]
    fn capability_tables() {
        assert!(supports_rephrase("EN-US"));
        assert!(!supports_rephrase("PL"));
        assert!(supports_formality("PL"));
        assert!(!supports_formality("EN-US"));
    }
}
