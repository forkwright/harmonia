//! BCP 47 language tag handling: normalization and fallback chains.

/// Normalize a language tag to a canonical BCP 47 form.
///
/// Converts ISO 639-2 three-letter codes to two-letter ISO 639-1 equivalents
/// and lowercases the result.
pub(crate) fn normalize(tag: &str) -> String {
    let lower = tag.to_ascii_lowercase();
    // Map common ISO 639-2 codes to ISO 639-1.
    match lower.as_str() {
        "eng" => "en".to_string(),
        "fre" | "fra" => "fr".to_string(),
        "ger" | "deu" => "de".to_string(),
        "spa" => "es".to_string(),
        "ita" => "it".to_string(),
        "por" => "pt".to_string(),
        "jpn" => "ja".to_string(),
        "kor" => "ko".to_string(),
        "chi" | "zho" => "zh".to_string(),
        "dut" | "nld" => "nl".to_string(),
        "swe" => "sv".to_string(),
        "nor" => "no".to_string(),
        "dan" => "da".to_string(),
        "fin" => "fi".to_string(),
        "pol" => "pl".to_string(),
        "rus" => "ru".to_string(),
        "ara" => "ar".to_string(),
        "tur" => "tr".to_string(),
        "ces" | "cze" => "cs".to_string(),
        "hun" => "hu".to_string(),
        "ron" | "rum" => "ro".to_string(),
        "hrv" => "hr".to_string(),
        "heb" => "he".to_string(),
        "tha" => "th".to_string(),
        "ind" => "id".to_string(),
        "vie" => "vi".to_string(),
        other => other.to_string(),
    }
}

/// Normalize a list of language tags preserving order.
pub(crate) fn normalize_preferences(languages: &[String]) -> Vec<String> {
    languages.iter().map(|l| normalize(l)).collect()
}

/// Build a fallback chain for a BCP 47 tag.
///
/// "pt-BR" → ["pt-br", "pt"]
/// "en-US" → ["en-us", "en"]
/// "en"    → ["en"]
pub(crate) fn fallback_chain(tag: &str) -> Vec<String> {
    let normalized = normalize(tag);
    // `split_once('-')` succeeds only when a subtag separator is present.
    if let Some((base, _)) = normalized.split_once('-') {
        return vec![normalized.clone(), base.to_string()];
    }
    vec![normalized]
}

/// Return the preference-ordered list of languages that a given candidate tag
/// satisfies, including fallback matching.
///
/// Returns the index of the first preference the candidate satisfies (lower =
/// higher priority), or `None` if the candidate is not wanted.
pub fn preference_rank(candidate: &str, preferences: &[String]) -> Option<usize> {
    let normalized_candidate = normalize(candidate);
    let chain = fallback_chain(&normalized_candidate);

    for (i, pref) in preferences.iter().enumerate() {
        let normalized_pref = normalize(pref);
        if chain.contains(&normalized_pref) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eng_normalized_to_en() {
        assert_eq!(normalize("eng"), "en");
    }

    #[test]
    fn fre_normalized_to_fr() {
        assert_eq!(normalize("fre"), "fr");
    }

    #[test]
    fn fra_normalized_to_fr() {
        assert_eq!(normalize("fra"), "fr");
    }

    #[test]
    fn two_letter_codes_unchanged() {
        assert_eq!(normalize("en"), "en");
        assert_eq!(normalize("fr"), "fr");
        assert_eq!(normalize("de"), "de");
    }

    #[test]
    fn normalize_is_case_insensitive() {
        assert_eq!(normalize("ENG"), "en");
        assert_eq!(normalize("FRE"), "fr");
        assert_eq!(normalize("En"), "en");
    }

    #[test]
    fn fallback_pt_br_includes_pt() {
        let chain = fallback_chain("pt-BR");
        assert_eq!(chain, vec!["pt-br".to_string(), "pt".to_string()]);
    }

    #[test]
    fn fallback_en_us_includes_en() {
        let chain = fallback_chain("en-US");
        assert_eq!(chain, vec!["en-us".to_string(), "en".to_string()]);
    }

    #[test]
    fn fallback_plain_tag_has_no_extra_entry() {
        let chain = fallback_chain("en");
        assert_eq!(chain, vec!["en".to_string()]);
    }

    #[test]
    fn preference_ordering_respected() {
        let prefs = vec!["en".to_string(), "fr".to_string(), "de".to_string()];
        assert_eq!(preference_rank("en", &prefs), Some(0));
        assert_eq!(preference_rank("fr", &prefs), Some(1));
        assert_eq!(preference_rank("de", &prefs), Some(2));
        assert_eq!(preference_rank("ja", &prefs), None);
    }

    #[test]
    fn fallback_pt_br_matches_pt_preference() {
        // If the user wants "pt" and the candidate is "pt-br", it should match.
        let prefs = vec!["pt".to_string()];
        // pt-br falls back to pt, so rank should be Some(0)
        let rank = preference_rank("pt-br", &prefs);
        assert_eq!(rank, Some(0));
    }

    #[test]
    fn preference_rank_uses_normalized_three_letter_codes() {
        let prefs = vec!["en".to_string(), "fr".to_string()];
        assert_eq!(preference_rank("eng", &prefs), Some(0));
        assert_eq!(preference_rank("fre", &prefs), Some(1));
    }
}
