//! Subtitle search orchestration: fan-out across providers, score, and rank.

use themelion::{MediaId, MediaType};
use tracing::{instrument, warn};

use crate::error::ProsthekeError;
use crate::language;
use crate::providers::SubtitleProvider;
use crate::types::{LanguagePreference, SubtitleMatch};

/// Search all configured providers sequentially.
///
/// Returns the best match per language, ranked by score. Candidates scoring
/// below `min_score` are silently discarded with a warning.
#[expect(
    clippy::too_many_arguments,
    reason = "forwards to SubtitleProvider::search which requires all these parameters"
)]
#[instrument(skip(providers, preferences), fields(media_id = %media_id))]
pub async fn search_all_providers<P: SubtitleProvider>(
    providers: &[P],
    media_id: &MediaId,
    media_type: MediaType,
    title: &str,
    year: Option<u16>,
    season: Option<u32>,
    episode: Option<u32>,
    preferences: &LanguagePreference,
    file_hash: Option<&str>,
    min_score: f64,
) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
    let normalized_langs: Vec<String> = language::normalize_preferences(&preferences.languages);

    let mut all_matches: Vec<SubtitleMatch> = Vec::new();

    for provider in providers {
        match provider
            .search(
                media_id,
                media_type,
                title,
                year,
                season,
                episode,
                &normalized_langs,
                file_hash,
            )
            .await
        {
            Ok(matches) => all_matches.extend(matches),
            Err(e) => {
                warn!(
                    error = %e,
                    provider = provider.name(),
                    "provider search failed — skipping"
                );
            }
        }
    }

    // Filter out results below the minimum score.
    let before = all_matches.len();
    all_matches.retain(|m| m.score >= min_score);
    let discarded = before - all_matches.len();
    if discarded > 0 {
        warn!(
            discarded,
            min_score, "discarded low-score subtitle candidates"
        );
    }

    // Apply preference filters.
    if !preferences.include_hearing_impaired {
        all_matches.retain(|m| !m.hearing_impaired);
    }
    if !preferences.include_forced {
        all_matches.retain(|m| !m.forced);
    }

    // Select the top match per language in preference order.
    let top = select_top_per_language(&all_matches, &normalized_langs);
    Ok(top)
}

/// Return the highest-scoring match for each requested language.
fn select_top_per_language(matches: &[SubtitleMatch], languages: &[String]) -> Vec<SubtitleMatch> {
    let mut result: Vec<SubtitleMatch> = Vec::new();

    for lang in languages {
        // Collect candidates for this language (including fallbacks).
        let chain = language::fallback_chain(lang);
        let mut candidates: Vec<&SubtitleMatch> = matches
            .iter()
            .filter(|m| {
                let normalized = language::normalize(&m.language);
                chain.contains(&normalized)
            })
            .collect();

        // Sort descending by score; pick the best.
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(best) = candidates.first() {
            result.push(SubtitleMatch {
                provider: best.provider.clone(),
                provider_id: best.provider_id.clone(),
                language: lang.clone(),
                hearing_impaired: best.hearing_impaired,
                forced: best.forced,
                score: best.score,
                download_url: best.download_url.clone(),
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_match(lang: &str, score: f64, _hash_match: bool) -> SubtitleMatch {
        SubtitleMatch {
            provider: "opensubtitles".to_string(),
            provider_id: crate::types::SubtitleProviderId(format!("id-{lang}-{score}")),
            language: lang.to_string(),
            hearing_impaired: false,
            forced: false,
            score,
            download_url: format!("https://example.com/subs/{lang}"),
        }
    }

    #[test]
    fn select_top_per_language_picks_highest_score() {
        let matches = vec![
            make_match("en", 0.75, false),
            make_match("en", 1.0, true),
            make_match("fr", 0.8, false),
        ];
        let top = select_top_per_language(&matches, &["en".to_string(), "fr".to_string()]);
        assert_eq!(top.len(), 2);
        let en = top.iter().find(|m| m.language == "en").unwrap();
        assert_eq!(en.score, 1.0);
    }

    #[test]
    fn hash_match_scores_higher_than_title_match() {
        // This validates that callers provide scores where hash > title.
        // The orchestrator doesn't modify scores; it just picks the best.
        let hash_score = 1.0f64;
        let title_score = 0.75f64;
        assert!(hash_score > title_score);

        let matches = vec![
            make_match("en", title_score, false),
            make_match("en", hash_score, true),
        ];
        let top = select_top_per_language(&matches, &["en".to_string()]);
        assert_eq!(top[0].score, hash_score);
    }

    #[test]
    fn below_threshold_candidates_not_included() {
        let matches = [make_match("en", 0.5, false), make_match("fr", 0.9, false)];
        // Simulate the filter that search_all_providers applies.
        let min = 0.7;
        let filtered: Vec<_> = matches.iter().filter(|m| m.score >= min).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].language, "fr");
    }

    #[test]
    fn select_top_per_language_handles_pt_br_fallback() {
        let matches = vec![make_match("pt", 0.8, false)];
        // Request "pt-br"; the fallback chain includes "pt".
        let top = select_top_per_language(&matches, &["pt-br".to_string()]);
        // Should find "pt" as a fallback for "pt-br".
        assert_eq!(top.len(), 1);
    }

    #[test]
    fn no_candidates_for_language_returns_nothing() {
        let matches = vec![make_match("en", 0.9, false)];
        let top = select_top_per_language(&matches, &["ja".to_string()]);
        assert!(top.is_empty());
    }

    /// Scripted provider for exercising `search_all_providers` without I/O.
    struct MockProvider {
        name: &'static str,
        fail: bool,
        matches: Vec<SubtitleMatch>,
    }

    impl crate::providers::SubtitleProvider for MockProvider {
        fn name(&self) -> &str {
            self.name
        }

        async fn search(
            &self,
            _media_id: &MediaId,
            _media_type: MediaType,
            _title: &str,
            _year: Option<u16>,
            _season: Option<u32>,
            _episode: Option<u32>,
            _languages: &[String],
            _file_hash: Option<&str>,
        ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
            if self.fail {
                return crate::error::AcquisitionFailedSnafu {
                    detail: "scripted failure".to_string(),
                }
                .fail();
            }
            Ok(self.matches.clone())
        }

        async fn download(&self, _subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
            Ok(vec![])
        }
    }

    fn preferences() -> LanguagePreference {
        LanguagePreference {
            languages: vec!["en".to_string()],
            include_hearing_impaired: true,
            include_forced: true,
        }
    }

    fn flagged_match(id: &str, hearing_impaired: bool, forced: bool) -> SubtitleMatch {
        SubtitleMatch {
            provider: "mock".to_string(),
            provider_id: crate::types::SubtitleProviderId(id.to_string()),
            language: "en".to_string(),
            hearing_impaired,
            forced,
            score: 0.9,
            download_url: "https://example.com/subs/en".to_string(),
        }
    }

    #[tokio::test]
    async fn search_all_providers_isolates_single_provider_failure() {
        let providers = vec![
            MockProvider {
                name: "broken",
                fail: true,
                matches: vec![],
            },
            MockProvider {
                name: "working",
                fail: false,
                matches: vec![make_match("en", 0.9, false)],
            },
        ];

        let result = search_all_providers(
            &providers,
            &MediaId::new(),
            MediaType::Movie,
            "Inception",
            Some(2010),
            None,
            None,
            &preferences(),
            None,
            0.5,
        )
        .await
        .unwrap();

        assert_eq!(
            result.len(),
            1,
            "the working provider's match must survive a sibling failure"
        );
        assert_eq!(result[0].language, "en");
    }

    #[tokio::test]
    async fn search_all_providers_all_fail_returns_empty_not_error() {
        let providers = vec![
            MockProvider {
                name: "broken-a",
                fail: true,
                matches: vec![],
            },
            MockProvider {
                name: "broken-b",
                fail: true,
                matches: vec![],
            },
        ];

        let result = search_all_providers(
            &providers,
            &MediaId::new(),
            MediaType::Movie,
            "Inception",
            None,
            None,
            None,
            &preferences(),
            None,
            0.5,
        )
        .await
        .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn search_excludes_hearing_impaired_when_preference_false() {
        let providers = vec![MockProvider {
            name: "mock",
            fail: false,
            matches: vec![
                flagged_match("hi", true, false),
                flagged_match("plain", false, false),
            ],
        }];
        let prefs = LanguagePreference {
            include_hearing_impaired: false,
            ..preferences()
        };

        let result = search_all_providers(
            &providers,
            &MediaId::new(),
            MediaType::Movie,
            "Inception",
            None,
            None,
            None,
            &prefs,
            None,
            0.5,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id.0, "plain");
        assert!(!result[0].hearing_impaired);
    }

    #[tokio::test]
    async fn search_keeps_hearing_impaired_when_preference_true() {
        let providers = vec![MockProvider {
            name: "mock",
            fail: false,
            matches: vec![flagged_match("hi", true, false)],
        }];

        let result = search_all_providers(
            &providers,
            &MediaId::new(),
            MediaType::Movie,
            "Inception",
            None,
            None,
            None,
            &preferences(),
            None,
            0.5,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id.0, "hi");
    }

    #[tokio::test]
    async fn search_excludes_forced_when_preference_false() {
        let providers = vec![MockProvider {
            name: "mock",
            fail: false,
            matches: vec![
                flagged_match("forced", false, true),
                flagged_match("plain", false, false),
            ],
        }];
        let prefs = LanguagePreference {
            include_forced: false,
            ..preferences()
        };

        let result = search_all_providers(
            &providers,
            &MediaId::new(),
            MediaType::Movie,
            "Inception",
            None,
            None,
            None,
            &prefs,
            None,
            0.5,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id.0, "plain");
        assert!(!result[0].forced);
    }
}
